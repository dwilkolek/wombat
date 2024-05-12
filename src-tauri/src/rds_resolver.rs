use crate::{aws, cache_db};
use log::info;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

const CACHE_NAME: &str = "rds";

pub struct RdsResolver {
    db: Arc<RwLock<libsql::Database>>,
    aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
}

impl RdsResolver {
    pub fn new(
        db: Arc<RwLock<libsql::Database>>,
        aws_config_resolver: Arc<RwLock<aws::AwsConfigProvider>>,
    ) -> Self {
        RdsResolver {
            db,
            aws_config_resolver,
        }
    }
    pub async fn init(&mut self, db: Arc<RwLock<libsql::Database>>) {
        {
            info!("initializing rds resolver, migratiob");
            let db = db.read().await;
            let conn = db.connect().unwrap();
            RdsResolver::migrate(&conn).await;
        }

        self.db = db;
    }

    async fn migrate(conn: &libsql::Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME).await;
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
                (),
            )
            .await
            .unwrap();
            cache_db::set_cache_version(conn, CACHE_NAME, 1).await;
        }
    }

    pub async fn refresh(&mut self) -> Vec<aws::RdsInstance> {
        {
            let db = self.db.read().await;
            clear_databases(&db.connect().unwrap()).await;
        }
        self.databases().await
    }

    pub async fn databases(&mut self) -> Vec<aws::RdsInstance> {
        info!("Resolving databases");
        let mut aws_config_resolver = self.aws_config_resolver.write().await;
        let environments = aws_config_resolver.configured_envs();
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let rdses = fetch_databases(&conn).await;
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

        store_databases(&conn, &databases).await;
        info!(
            "Returning databases from aws and persisting, count: {}",
            databases.len()
        );
        databases
    }

    pub async fn read_databases(&self) -> Vec<aws::RdsInstance> {
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let databases = fetch_databases(&conn).await;
        let environments = self.aws_config_resolver.read().await.configured_envs();
        databases
            .into_iter()
            .filter(|cluster| environments.contains(&cluster.env))
            .collect()
    }
}

async fn fetch_databases(conn: &libsql::Connection) -> Vec<aws::RdsInstance> {
    log::info!("reading rds instances from cache");
    let result = conn
        .query(
            "SELECT arn, name, engine, engine_version, endpoint, environment_tag, env, appname_tag FROM databases;",
            (),
        )
        .await;
    match result {
        Ok(mut rows) => {
            let mut databases = Vec::new();
            while let Ok(row) = rows.next().await {
                match row {
                    Some(row) => {
                        let arn = row.get::<String>(0).unwrap();
                        let name = row.get::<String>(1).unwrap();
                        let engine = row.get::<String>(2).unwrap();
                        let engine_version = row.get::<String>(3).unwrap();
                        let endpoint =
                            serde_json::from_str(&row.get::<String>(4).unwrap()).unwrap();
                        let environment_tag = row.get::<String>(5).unwrap();
                        let env = serde_json::from_str(&row.get::<String>(6).unwrap()).unwrap();
                        let appname_tag = row.get::<String>(7).unwrap();
                        databases.push(aws::RdsInstance {
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
                    }
                    None => {
                        break;
                    }
                }
            }
            log::info!("read {} rds instances from cache", databases.len());
            databases
        }
        Err(e) => {
            log::error!("reading rds instances from cache failed, reason: {}", e);
            Vec::new()
        }
    }
}
async fn clear_databases(conn: &libsql::Connection) {
    info!("dropping rds instances from cache");
    conn.execute("DELETE FROM databases", ()).await.unwrap();
}

async fn store_databases(conn: &libsql::Connection, databases: &[aws::RdsInstance]) {
    clear_databases(conn).await;

    for db in databases.iter() {
        conn.execute(
                "INSERT INTO
                    databases(arn, name, engine, engine_version, endpoint, environment_tag, env, appname_tag)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ",
                libsql::params![
                    db.arn.clone(),
                    db.name.clone(),
                    db.engine.clone(),
                    db.engine_version.clone(),
                    serde_json::to_string(&db.endpoint).unwrap(),
                    db.environment_tag.clone(),
                    serde_json::to_string(&db.env).unwrap(),
                    db.appname_tag.clone()
                ],
            ).await.unwrap();
    }

    info!("stored {} rds instances in cache", databases.len());
}
