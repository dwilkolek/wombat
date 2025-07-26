use crate::shared;
use crate::{aws, cache_db};
use log::info;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

const CACHE_NAME: &str = "cluster";

pub struct ClusterResolver {
    db_pool: Arc<Pool<SqliteConnectionManager>>,
    aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
}

impl ClusterResolver {
    pub fn new(
        db_pool: Arc<Pool<SqliteConnectionManager>>,
        aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
    ) -> Self {
        ClusterResolver {
            db_pool,
            aws_config_resolver,
        }
    }
    pub fn init(&mut self, db_pool: Arc<Pool<SqliteConnectionManager>>) {
        let pool = db_pool.clone();
        tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            Self::migrate(&conn);
        });
        self.db_pool = db_pool;
    }
    fn migrate(conn: &Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME);
        if version < 1 {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS clusters(
                            arn TEXT PRIMARY KEY NOT NULL,
                            name TEXT NOT NULL,
                            env TEXT NOT NULL
                        )",
                [],
            )
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 1);
        }
        if version < 2 {
            conn.execute(
                "ALTER TABLE clusters
                  ADD COLUMN platform_version int default 0
                  ",
                [],
            )
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 2);
        }
    }

    pub async fn refresh(&mut self) -> Vec<aws::Cluster> {
        let pool = self.db_pool.clone();
        tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            clear_clusters(&conn);
        });
        self.clusters().await.clone()
    }

    pub async fn clusters(&mut self) -> Vec<aws::Cluster> {
        let aws_config_resolver = self.aws_config_resolver.read().await;
        let environments = aws_config_resolver.configured_envs();
        info!("Resolving clusters");
        let pool = self.db_pool.clone();
        let clusters = tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            fetch_clusters(&conn)
        });
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

        tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            store_clusters(&conn, &clusters);
        });
        info!(
            "Returning clusters from aws and persisting, count: {}",
            clusters.len()
        );
        clusters
    }

    pub async fn read_clusters(&self) -> Vec<aws::Cluster> {
        info!("Resolving clusters");
        let pool = self.db_pool.clone();
        let clusters = tokio::task::block_in_place(|| {
            let conn = pool.get().unwrap();
            fetch_clusters(&conn)
        });
        let environments = self.aws_config_resolver.read().await.configured_envs();
        clusters
            .into_iter()
            .filter(|cluster| environments.contains(&cluster.env))
            .collect()
    }
}

fn fetch_clusters(conn: &Connection) -> Vec<aws::Cluster> {
    log::info!("reading clusters from cache");
    let mut stmt = match conn.prepare("SELECT arn, name, env, platform_version FROM clusters;") {
        Ok(s) => s,
        Err(e) => {
            log::error!("reading clusters cache failed, reason: {e}");
            return Vec::new();
        }
    };
    let rows = stmt.query_map([], |row| {
        let arn: String = row.get(0)?;
        let name: String = row.get(1)?;
        let env: shared::Env = serde_json::from_str(&row.get::<usize, String>(2)?).unwrap();
        let platform_version: i32 = row.get(3).unwrap_or(0);
        Ok(aws::Cluster {
            arn,
            name,
            env,
            platform_version,
        })
    });
    match rows {
        Ok(mapped) => {
            let clusters: Vec<_> = mapped.filter_map(Result::ok).collect();
            log::info!("read {} clusters from cache", clusters.len());
            clusters
        }
        Err(e) => {
            log::error!("reading clusters cache failed, reason: {e}");
            Vec::new()
        }
    }
}
fn clear_clusters(conn: &Connection) {
    info!("dropping clusters from cache");
    conn.execute("DELETE FROM clusters", []).unwrap();
}
fn store_clusters(conn: &Connection, clusters: &[aws::Cluster]) {
    clear_clusters(conn);
    for db in clusters.iter() {
        conn.execute(
            "INSERT INTO clusters(arn, name, env) VALUES (?, ?, ?)",
            params![
                db.arn.clone(),
                db.name.clone(),
                serde_json::to_string(&db.env).unwrap()
            ],
        )
        .unwrap();
    }
    info!("stored {} clusters in cache", clusters.len());
}
