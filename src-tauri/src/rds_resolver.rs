use crate::{aws, cache_db};
use log::info;
use std::sync::Arc;
use tokio::sync::RwLock;

const CACHE_NAME: &str = "rds";

pub struct RdsResolver {
    db: Arc<RwLock<libsql::Database>>,
}

impl RdsResolver {
    pub async fn new(db: Arc<RwLock<libsql::Database>>) -> Self {
        {
            let db = db.read().await;
            let conn = db.connect().unwrap();
            RdsResolver::migrate(&conn).await;
        }

        RdsResolver { db }
    }

    async fn migrate(conn: &libsql::Connection) {
        let version = cache_db::get_cache_version(conn, CACHE_NAME).await;

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

    pub async fn refresh(&mut self, config: &aws_config::SdkConfig) -> Vec<aws::RdsInstance> {
        {
            let db = self.db.read().await;
            clear_databases(&db.connect().unwrap()).await;
        }
        return self.databases(config).await;
    }

    pub async fn databases(&mut self, config: &aws_config::SdkConfig) -> Vec<aws::RdsInstance> {
        info!("Resolving databases");
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        let rdses = fetch_databases(&conn).await;
        if rdses.len() > 0 {
            info!("Returning databases from cache");
            return rdses.clone();
        }
        info!("Fetching rds from aws");
        let fresh_databases = aws::databases(config).await;
        store_databases(&conn, &fresh_databases).await;

        info!(
            "Returning databases from aws and persisting, count: {}",
            fresh_databases.len()
        );
        return fresh_databases;
    }

    pub async fn read_databases(&self) -> Vec<aws::RdsInstance> {
        let db = self.db.read().await;
        let conn = db.connect().unwrap();
        return fetch_databases(&conn).await;
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

async fn store_databases(conn: &libsql::Connection, databases: &Vec<aws::RdsInstance>) {
    clear_databases(&conn).await;

    for db in databases.into_iter() {
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
