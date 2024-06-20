use crate::{aws, cache_db};
use log::info;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

const CACHE_NAME: &str = "cluster";

pub struct ClusterResolver {
    db: Arc<RwLock<libsql::Database>>,
    aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
}

impl ClusterResolver {
    pub fn new(
        db: Arc<RwLock<libsql::Database>>,
        aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
    ) -> Self {
        ClusterResolver {
            db,
            aws_config_resolver,
        }
    }
    pub async fn init(&mut self, db: Arc<RwLock<libsql::Database>>) {
        {
            let db = db.read().await;
            let conn = db.connect().unwrap();
            ClusterResolver::migrate(&conn).await;
        }

        self.db = db;
    }
    async fn migrate(conn: &libsql::Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME).await;

        if version < 1 {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS clusters(
                            arn TEXT PRIMARY KEY NOT NULL,
                            name TEXT NOT NULL,
                            env TEXT NOT NULL
                        )",
                (),
            )
            .await
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 1).await;
        }
    }

    pub async fn refresh(&mut self) -> Vec<aws::Cluster> {
        {
            let db = self.db.read().await;
            clear_clusters(&db.connect().unwrap()).await;
        }
        self.clusters().await.clone()
    }

    pub async fn clusters(&mut self) -> Vec<aws::Cluster> {
        let aws_config_resolver = self.aws_config_resolver.read().await;
        let environments = aws_config_resolver.configured_envs();
        info!("Resolving clusters");
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let clusters = fetch_clusters(&conn).await;
        if !clusters.is_empty() {
            info!("Returning clusters from cache");
            return clusters
                .into_iter()
                .filter(|cluster| environments.iter().any(|env| env == &cluster.env))
                .collect();
        }

        let mut unique_clusters_map = HashMap::new();
        for env in environments.iter() {
            let (profile, config) = aws_config_resolver.sso_config(env).await;
            info!("Fetching ecs from aws using {profile}");
            let clusters = aws::clusters(&config).await;
            for cluster in clusters {
                unique_clusters_map.insert(cluster.arn.clone(), cluster);
            }
        }

        let clusters = unique_clusters_map.values().cloned().collect::<Vec<_>>();

        store_clusters(&conn, &clusters).await;
        info!(
            "Returning clusters from aws and persisting, count: {}",
            clusters.len()
        );
        clusters
    }

    pub async fn read_clusters(&self) -> Vec<aws::Cluster> {
        info!("Resolving clusters");
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let clusters = fetch_clusters(&conn).await;
        let environments = self.aws_config_resolver.read().await.configured_envs();
        clusters
            .into_iter()
            .filter(|cluster| environments.contains(&cluster.env))
            .collect()
    }
}

async fn fetch_clusters(conn: &libsql::Connection) -> Vec<aws::Cluster> {
    log::info!("reading clusters from cache");
    let result = conn.query("SELECT arn, name, env FROM clusters;", ()).await;
    match result {
        Ok(mut rows) => {
            let mut clusters = Vec::new();
            while let Ok(row) = rows.next().await {
                match row {
                    Some(row) => {
                        let arn = row.get::<String>(0).unwrap();
                        let name = row.get::<String>(1).unwrap();
                        let env = serde_json::from_str(&row.get::<String>(2).unwrap()).unwrap();
                        clusters.push(aws::Cluster { arn, name, env })
                    }
                    None => {
                        break;
                    }
                }
            }
            log::info!("read {} clusters from cache", clusters.len());
            clusters
        }
        Err(e) => {
            log::error!("reading clusters cache failed, reason: {}", e);
            Vec::new()
        }
    }
}
async fn clear_clusters(conn: &libsql::Connection) {
    info!("dropping clusters from cache");
    conn.execute("DELETE FROM clusters", ()).await.unwrap();
}

async fn store_clusters(conn: &libsql::Connection, clusters: &[aws::Cluster]) {
    clear_clusters(conn).await;

    for db in clusters.iter() {
        conn.execute(
            "INSERT INTO
                    clusters(arn, name, env)
                    VALUES (?, ?, ?)
                ",
            libsql::params![
                db.arn.clone(),
                db.name.clone(),
                serde_json::to_string(&db.env).unwrap()
            ],
        )
        .await
        .unwrap();
    }

    info!("stored {} clusters in cache", clusters.len());
}
