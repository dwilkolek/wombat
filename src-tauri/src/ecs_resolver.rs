use crate::{aws, cache_db, shared::BError};
use log::info;
use std::{collections::HashMap, sync::Arc};
use tauri::Window;
use tokio::sync::RwLock;

const CACHE_NAME: &str = "ecs";

pub struct EcsResolver {
    db: Arc<RwLock<libsql::Database>>,
    aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
}

impl EcsResolver {
    pub fn new(
        db: Arc<RwLock<libsql::Database>>,
        aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
    ) -> Self {
        EcsResolver {
            db,
            aws_config_resolver,
        }
    }
    pub async fn init(&mut self, db: Arc<RwLock<libsql::Database>>) {
        {
            let db = db.read().await;
            let conn = db.connect().unwrap();
            EcsResolver::migrate(&conn).await;
        }

        self.db = db;
    }

    async fn migrate(conn: &libsql::Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME).await;

        if version < 1 {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS services(
                            arn TEXT PRIMARY KEY NOT NULL,
                            name TEXT NOT NULL,
                            cluster_arn TEXT NOT NULL,
                            env TEXT NOT NULL
                        )",
                (),
            )
            .await
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 1).await;
        }
    }

    pub async fn refresh(&mut self, clusters: Vec<aws::Cluster>) -> Vec<aws::EcsService> {
        {
            let db = self.db.read().await;
            clear_services(&db.connect().unwrap()).await;
        }
        self.services(clusters).await
    }

    pub async fn restart_service(
        &self,
        window: Window,
        config: aws_config::SdkConfig,
        cluster_arn: String,
        service_name: String,
    ) -> Result<String, BError> {
        let deplyoment_res = aws::restart_service(&config, &cluster_arn, &service_name).await;

        if deplyoment_res.is_ok() {
            let deployment_res_clone = deplyoment_res.clone();
            let deployment_id = deployment_res_clone.unwrap().clone();
            tokio::task::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(5000));
                let mut continue_checking = true;
                while continue_checking {
                    interval.tick().await;
                    let status = aws::get_deploment_status(
                        &config,
                        &cluster_arn,
                        &service_name,
                        &deployment_id,
                    )
                    .await;
                    let mut status_str = "Unknown";
                    if let Ok(status) = status {
                        status_str = match status {
                            aws_sdk_ecs::types::DeploymentRolloutState::Completed => "Completed",
                            aws_sdk_ecs::types::DeploymentRolloutState::Failed => "Failed",
                            aws_sdk_ecs::types::DeploymentRolloutState::InProgress => "In Progress",
                            _ => "Unknown",
                        };
                    }

                    let _ = window.emit(
                        "deployment",
                        DeplyomentStatus {
                            deployment_id: deployment_id.to_owned(),
                            cluster_arn: cluster_arn.clone(),
                            service_name: service_name.clone(),
                            rollout_status: status_str.to_owned(),
                        },
                    );
                    continue_checking = status_str == "In Progress";
                }
            });
        }

        deplyoment_res
    }

    pub async fn services(&mut self, clusters: Vec<aws::Cluster>) -> Vec<aws::EcsService> {
        info!("Resolving services for clusters {clusters:?}");
        let mut aws_config_resolver = self.aws_config_resolver.write().await;
        let environments = aws_config_resolver.configured_envs();
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let ecses = fetch_services(&conn).await;
        if !ecses.is_empty() {
            info!("Returning services from cache");
            return ecses
                .into_iter()
                .filter(|ecs| {
                    clusters
                        .iter()
                        .any(|cluster| cluster.arn == ecs.cluster_arn)
                })
                .collect();
        }

        let mut unique_services_map = HashMap::new();
        for env in environments.iter() {
            let (profile, config) = aws_config_resolver.user_config(env).await;
            info!("Fetching ecs from aws using {profile}");
            let services = aws::services(&config, &clusters).await;
            for service in services {
                if clusters
                    .iter()
                    .any(|cluster| cluster.arn == service.cluster_arn)
                {
                    unique_services_map.insert(service.arn.clone(), service);
                }
            }
        }

        let services = unique_services_map.values().cloned().collect::<Vec<_>>();

        store_services(&conn, &services).await;
        info!(
            "Returning services from aws and persisting, count: {}",
            services.len()
        );
        services
    }

    pub async fn read_services(&self) -> Vec<aws::EcsService> {
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let services = fetch_services(&conn).await;
        let environments = self.aws_config_resolver.read().await.configured_envs();
        services
            .into_iter()
            .filter(|cluster| environments.contains(&cluster.env))
            .collect()
    }
}

async fn fetch_services(conn: &libsql::Connection) -> Vec<aws::EcsService> {
    log::info!("reading ecs instances from cache");
    let result = conn
        .query("SELECT arn, name, cluster_arn, env FROM services", ())
        .await;
    match result {
        Ok(mut rows) => {
            let mut services = Vec::new();
            while let Ok(row) = rows.next().await {
                match row {
                    Some(row) => {
                        let arn = row.get::<String>(0).unwrap();
                        let name = row.get::<String>(1).unwrap();
                        let cluster_arn = row.get::<String>(2).unwrap();
                        let env = serde_json::from_str(&row.get::<String>(3).unwrap()).unwrap();
                        services.push(aws::EcsService {
                            arn,
                            name,
                            cluster_arn,
                            env,
                        })
                    }
                    None => {
                        break;
                    }
                }
            }
            log::info!("read {} ecs instances from cache", services.len());
            services
        }
        Err(e) => {
            log::error!("reading ecs instances from cache failed, reason: {}", e);
            Vec::new()
        }
    }
}
async fn clear_services(conn: &libsql::Connection) {
    info!("dropping ecs instances from cache");
    conn.execute("DELETE FROM services", ()).await.unwrap();
}

async fn store_services(conn: &libsql::Connection, services: &[aws::EcsService]) {
    clear_services(conn).await;

    for ecs in services.iter() {
        conn.execute(
            "INSERT INTO
                    services(arn, name, cluster_arn, env)
                    VALUES (?, ?, ?, ?)
                ",
            libsql::params![
                ecs.arn.clone(),
                ecs.name.clone(),
                ecs.cluster_arn.clone(),
                serde_json::to_string(&ecs.env).unwrap(),
            ],
        )
        .await
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
}
