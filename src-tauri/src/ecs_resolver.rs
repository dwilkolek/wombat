use crate::{
    aws, cache_db,
    shared::{self, arn_to_name, BError, Env},
};
use log::{info, warn};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite;
use std::{collections::HashMap, sync::Arc};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

const CACHE_NAME: &str = "ecs";

pub struct EcsResolver {
    db_pool: Arc<Pool<SqliteConnectionManager>>,
    aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
}

impl EcsResolver {
    pub fn new(
        db_pool: Arc<Pool<SqliteConnectionManager>>,
        aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
    ) -> Self {
        EcsResolver {
            db_pool,
            aws_config_resolver,
        }
    }
    pub async fn init(&mut self, db_pool: Arc<Pool<SqliteConnectionManager>>) {
        let pool = db_pool.clone();
        tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            Self::migrate(&conn);
        });
        self.db_pool = db_pool;
    }
    fn migrate(conn: &rusqlite::Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME);
        if version < 1 {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS services(
                            arn TEXT PRIMARY KEY NOT NULL,
                            name TEXT NOT NULL,
                            cluster_arn TEXT NOT NULL,
                            env TEXT NOT NULL
                        )",
                [],
            )
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 1);
        }
        if version < 2 {
            conn.execute(
                "ALTER TABLE services
                  ADD COLUMN td_family text default ''
                  ",
                [],
            )
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 2);
        }
    }

    pub async fn refresh(&mut self, clusters: Vec<aws::Cluster>) -> Vec<aws::EcsService> {
        {
            let pool = self.db_pool.clone();
            tokio::task::block_in_place(|| {
                let conn = pool.get().unwrap();
                clear_services(&conn);
            });
        }
        self.services(clusters).await
    }

    pub async fn deploy_service(
        &self,
        app_handle: AppHandle,
        config: aws_config::SdkConfig,
        cluster_arn: String,
        service_arn: String,
        desired_version: Option<String>,
        include_terraform_tag: bool,
    ) -> Result<String, BError> {
        let deplyoment_res = aws::deploy_service(
            &config,
            &cluster_arn,
            &service_arn,
            desired_version.clone(),
            include_terraform_tag,
        )
        .await;
        if deplyoment_res.is_ok() {
            let deployment_res_clone = deplyoment_res.clone();
            let deployment_id = deployment_res_clone.unwrap().clone();
            let mut error_count = 0;
            let fail_deploy_after = chrono::Utc::now() + chrono::Duration::minutes(15);

            tokio::task::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(5000));
                let mut continue_checking = true;
                let service_name = arn_to_name(&service_arn);
                while continue_checking {
                    interval.tick().await;
                    info!("Checking deployment={deployment_id}");
                    let deployment_status = aws::get_deploment_status(
                        &config,
                        &cluster_arn,
                        &service_name,
                        &deployment_id,
                    )
                    .await;
                    info!(
                        "Deployment={} has status={:?}",
                        &deployment_id, &deployment_status
                    );
                    let mut status_str = "Unknown";
                    let mut error_message = None;
                    if let Ok(status) = deployment_status {
                        status_str = match status {
                            aws_sdk_ecs::types::DeploymentRolloutState::Completed => "Completed",
                            aws_sdk_ecs::types::DeploymentRolloutState::Failed => "Failed",
                            aws_sdk_ecs::types::DeploymentRolloutState::InProgress => "In Progress",
                            _ => {
                                error_count += 1;
                                error_message = Some(format!("Unknown status: {:?}", status));
                                "Unknown"
                            }
                        };
                    } else {
                        error_count += 1;
                        error_message = Some(deployment_status.unwrap_err().message.clone())
                    }

                    if error_count == 5 {
                        status_str = "Failed";
                        error_message = Some("Exceeded error count".to_owned())
                    }

                    if chrono::Utc::now() > fail_deploy_after {
                        status_str = "Failed";
                        error_message = Some("Timed out after 15m. Check in AWS console".to_owned())
                    }

                    let _ = app_handle.emit(
                        "deployment",
                        DeplyomentStatus {
                            deployment_id: deployment_id.to_owned(),
                            cluster_arn: cluster_arn.clone(),
                            service_name: service_name.clone(),
                            rollout_status: status_str.to_owned(),
                            version: desired_version.clone(),
                            error_message,
                        },
                    );
                    continue_checking = status_str == "In Progress" || status_str == "Unknown";
                }
            });
        }

        deplyoment_res
    }

    pub async fn services(&mut self, clusters: Vec<aws::Cluster>) -> Vec<aws::EcsService> {
        info!("Resolving services for clusters {clusters:?}");
        let aws_config_resolver = self.aws_config_resolver.read().await;
        let environments = aws_config_resolver.configured_envs();
        let pool = self.db_pool.clone();
        let services: Vec<aws::EcsService> = tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            fetch_services(&conn)
        });
        if !services.is_empty() {
            info!("Returning services from cache");
            return services
                .into_iter()
                .filter(|ecs| {
                    clusters
                        .iter()
                        .any(|cluster| cluster.arn == ecs.cluster_arn)
                })
                .collect();
        }

        let mut unique_services_map = HashMap::new();
        for cluster in clusters {
            if environments.contains(&cluster.env) {
                let (profile, config) = aws_config_resolver.sso_config(&cluster.env).await;
                info!(
                    "Using profile={} to resolve services for cluster={}",
                    cluster.arn, profile
                );
                let services = aws::services(&config, &cluster).await;
                for service in services {
                    unique_services_map.insert(service.arn.clone(), service);
                }
            } else {
                warn!(
                    "Skipping cluster: {} cause not in configured environments",
                    cluster.arn
                )
            }
        }

        let services = unique_services_map.values().cloned().collect::<Vec<_>>();

        store_services(&pool, &services);
        info!(
            "Returning services from aws and persisting, count: {}",
            services.len()
        );
        services
    }

    pub async fn read_services(&self) -> Vec<aws::EcsService> {
        let pool = self.db_pool.clone();
        let services: Vec<aws::EcsService> = tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            fetch_services(&conn)
        });
        let environments = self.aws_config_resolver.read().await.configured_envs();
        services
            .into_iter()
            .filter(|cluster| environments.contains(&cluster.env))
            .collect()
    }
}

fn fetch_services(conn: &rusqlite::Connection) -> Vec<aws::EcsService> {
    log::info!("reading ecs instances from cache");
    let mut stmt = match conn.prepare("SELECT arn, name, cluster_arn, env, td_family FROM services")
    {
        Ok(s) => s,
        Err(e) => {
            log::error!("reading ecs instances from cache failed, reason: {}", e);
            return Vec::new();
        }
    };
    let rows = stmt.query_map([], |row| {
        let arn: String = row.get(0)?;
        let name: String = row.get(1)?;
        let cluster_arn: String = row.get(2)?;
        let env: Env = serde_json::from_str(&row.get::<usize, String>(3)?).unwrap();
        let td_family: String = row.get(4)?;
        Ok(aws::EcsService {
            arn,
            name,
            cluster_arn,
            env,
            td_family,
        })
    });
    match rows {
        Ok(mapped) => {
            let services: Vec<_> = mapped.filter_map(Result::ok).collect();
            log::info!("read {} ecs instances from cache", services.len());
            services
        }
        Err(e) => {
            log::error!("reading ecs instances from cache failed, reason: {}", e);
            Vec::new()
        }
    }
}
fn clear_services(conn: &rusqlite::Connection) {
    info!("dropping ecs instances from cache");
    conn.execute("DELETE FROM services", []).unwrap();
}

fn store_services(pool: &Pool<SqliteConnectionManager>, services: &[aws::EcsService]) {
    clear_services(&pool.get().unwrap());

    for ecs in services.iter() {
        let conn = pool.get().unwrap();
        conn.execute(
            "INSERT INTO services(arn, name, cluster_arn, env) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                ecs.arn.clone(),
                ecs.name.clone(),
                ecs.cluster_arn.clone(),
                serde_json::to_string(&ecs.env).unwrap(),
            ],
        )
        .unwrap();
    }

    info!("stored {} ecs instances in cache", services.len());
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DeplyomentStatus {
    deployment_id: String,
    service_name: String,
    cluster_arn: String,
    rollout_status: String,
    version: Option<String>,
    error_message: Option<String>,
}
