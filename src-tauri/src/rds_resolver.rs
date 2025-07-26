use crate::shared;
use crate::{aws, cache_db};
use log::info;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

const CACHE_NAME: &str = "rds";

pub struct RdsResolver {
    db_pool: Arc<Pool<SqliteConnectionManager>>,
    aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
}

impl RdsResolver {
    pub fn new(
        db_pool: Arc<Pool<SqliteConnectionManager>>,
        aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
    ) -> Self {
        RdsResolver {
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

    fn migrate(conn: &rusqlite::Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME);
        info!("Version {}", &version);
        if version < 1 {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS databases(
                            arn TEXT PRIMARY KEY NOT NULL,
                            name TEXT NOT NULL,
                            engine TEXT NOT NULL,
                            engine_version TEXT NOT NULL,
                            endpoint TEXT NOT NULL,
                            environment_tag TEXT NOT NULL,
                            env TEXT NOT NULL,
                            appname_tag TEXT NOT NULL
                        )",
                [],
            )
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 1);
        }
    }

    pub async fn refresh(&mut self) -> Vec<aws::RdsInstance> {
        {
            let pool = self.db_pool.clone();
            tokio::task::block_in_place(|| {
                let conn = pool.get().unwrap();
                clear_databases(&conn);
            });
        }
        self.databases().await
    }

    pub async fn databases(&mut self) -> Vec<aws::RdsInstance> {
        info!("Resolving databases");
        let aws_config_resolver = self.aws_config_resolver.read().await;
        let environments = aws_config_resolver.configured_envs();
        let pool = self.db_pool.clone();
        let conn = pool.get().unwrap();
        let rdses = fetch_databases(&conn);
        if !rdses.is_empty() {
            info!("Returning databases from cache");
            return rdses
                .into_iter()
                .filter(|rds| environments.iter().any(|env| env == &rds.env))
                .collect();
        }

        let mut unique_databases_map = HashMap::new();

        for env in environments.iter() {
            let (profile, config) = aws_config_resolver.sso_config(env).await;
            info!("Fetching rds from aws using profile {profile}");
            let databases = aws::databases(&config).await;
            for db in databases {
                unique_databases_map.insert(db.arn.clone(), db);
            }
        }
        let databases = unique_databases_map.values().cloned().collect::<Vec<_>>();

        store_databases(&conn, &databases);
        info!(
            "Returning databases from aws and persisting, count: {}",
            databases.len()
        );
        databases
    }

    pub async fn read_databases(&self) -> Vec<aws::RdsInstance> {
        let pool = self.db_pool.clone();
        let conn = pool.get().unwrap();
        let databases = fetch_databases(&conn);
        let environments = self.aws_config_resolver.read().await.configured_envs();
        databases
            .into_iter()
            .filter(|cluster| environments.contains(&cluster.env))
            .collect()
    }
}

fn fetch_databases(conn: &rusqlite::Connection) -> Vec<aws::RdsInstance> {
    log::info!("reading rds instances from cache");
    let mut stmt = match conn.prepare("SELECT arn, name, engine, engine_version, endpoint, environment_tag, env, appname_tag FROM databases;") {
        Ok(s) => s,
        Err(e) => {
            log::error!("reading rds instances from cache failed, reason: {e}");
            return Vec::new();
        }
    };
    let rows = stmt.query_map([], |row| {
        let arn: String = row.get(0)?;
        let name: String = row.get(1)?;
        let engine: String = row.get(2)?;
        let engine_version: String = row.get(3)?;
        let endpoint: aws::Endpoint = serde_json::from_str(&row.get::<usize, String>(4)?).unwrap();
        let environment_tag: String = row.get(5)?;
        let env: shared::Env = serde_json::from_str(&row.get::<usize, String>(6)?).unwrap();
        let appname_tag: String = row.get(7)?;
        Ok(aws::RdsInstance {
            arn,
            normalized_name: name.replace("-migrated", ""),
            name,
            engine,
            engine_version,
            endpoint,
            environment_tag,
            env,
            appname_tag,
        })
    });
    match rows {
        Ok(mapped) => {
            let databases: Vec<_> = mapped.filter_map(Result::ok).collect();
            log::info!("read {} rds instances from cache", databases.len());
            databases
        }
        Err(e) => {
            log::error!("reading rds instances from cache failed, reason: {e}");
            Vec::new()
        }
    }
}
fn clear_databases(conn: &rusqlite::Connection) {
    info!("dropping rds instances from cache");
    conn.execute("DELETE FROM databases", []).unwrap();
}

fn store_databases(conn: &rusqlite::Connection, databases: &[aws::RdsInstance]) {
    clear_databases(conn);

    for db in databases.iter() {
        conn.execute(
            "INSERT INTO databases(arn, name, engine, engine_version, endpoint, environment_tag, env, appname_tag) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                db.arn.clone(),
                db.name.clone(),
                db.engine.clone(),
                db.engine_version.clone(),
                serde_json::to_string(&db.endpoint).unwrap(),
                db.environment_tag.clone(),
                serde_json::to_string(&db.env).unwrap(),
                db.appname_tag.clone()
            ],
        ).unwrap();
    }

    info!("stored {} rds instances in cache", databases.len());
}
