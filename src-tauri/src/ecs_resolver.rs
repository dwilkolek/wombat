use crate::{aws, cache_db};
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

const CACHE_NAME: &str = "ecs";

pub struct EcsResolver {
    db: Arc<RwLock<libsql::Database>>,
}

impl EcsResolver {
    pub async fn new(db: Arc<RwLock<libsql::Database>>) -> Self {
        {
            let db = db.read().await;
            let conn = db.connect().unwrap();
            EcsResolver::migrate(&conn).await;
        }

        EcsResolver { db }
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

    pub async fn refresh(
        &mut self,
        config: &aws_config::SdkConfig,
        clusters: Vec<aws::Cluster>,
    ) -> Vec<aws::EcsService> {
        {
            let db = self.db.read().await;
            clear_services(&db.connect().unwrap()).await;
        }
        return self.services(config, clusters).await;
    }

    pub async fn services(
        &mut self,
        config: &aws_config::SdkConfig,
        clusters: Vec<aws::Cluster>,
    ) -> Vec<aws::EcsService> {
        info!("Resolving services");
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let ecses = fetch_services(&conn).await;
        if ecses.len() > 0 {
            info!("Returning services from cache");
            return ecses.clone();
        }

        let fresh_services = aws::services(config, clusters).await;
        store_services(&conn, &fresh_services).await;

        info!("Returning services from aws and persisting");
        return fresh_services;
    }

    pub async fn read_services(&self) -> Vec<aws::EcsService> {
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        return fetch_services(&conn).await;
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

async fn store_services(conn: &libsql::Connection, services: &Vec<aws::EcsService>) {
    clear_services(&conn).await;

    for ecs in services.into_iter() {
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
