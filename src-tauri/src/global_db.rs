use libsql::{params, Connection, Transaction};
use log::info;
use serde::{Deserialize, Serialize};

pub async fn migrate(conn: &Connection) {
    let mut current_version = get_db_version(conn).await;
    if current_version < 1 {
        let transaction = conn.transaction().await.unwrap();
        transaction.execute(
            "CREATE TABLE IF NOT EXISTS migrations(version INTEGER PRIMARY KEY, changeset TEXT)",
            (),
        )
        .await
        .unwrap();

        current_version = fin(current_version, transaction, "init").await;
    }

    if current_version < 2 {
        let transaction = conn.transaction().await.unwrap();
        transaction
            .execute(
                "CREATE TABLE IF NOT EXISTS features(name TEXT, enabled BOOLEAN)",
                (),
            )
            .await
            .unwrap();
        transaction
            .execute(
                "INSERT INTO features(name, enabled) VALUES ('wombat', true)",
                (),
            )
            .await
            .unwrap();
        current_version = fin(current_version, transaction, "add features table").await;
    }

    if current_version < 3 {
        // { $.level = "ERROR" }
        // { $.mdc.traceId = "TRACE_ID_UUID" }
        let transaction = conn.transaction().await.unwrap();
        transaction
            .execute(
                "CREATE TABLE IF NOT EXISTS log_filters(filter TEXT, label TEXT, services TEXT)",
                (),
            )
            .await
            .unwrap();
        current_version = fin(current_version, transaction, "add logs filter table").await;
    }

    if current_version < 4 {
        // from app, to app, env, jepsen url, api prefix, client id, secret arn
        let transaction = conn.transaction().await.unwrap();
        transaction
            .execute(
                "CREATE TABLE IF NOT EXISTS jepsen_configs(from_app TEXT, to_app TEXT, env TEXT, auth_api TEXT, api_path TEXT, api_name TEXT, client_id TEXT, secret_name TEXT)",
                (),
            )
            .await
            .unwrap();
        current_version = fin(current_version, transaction, "add proxy auth table").await;
    }

    if current_version < 5 {
        let transaction = conn.transaction().await.unwrap();
        transaction
            .execute(
                "ALTER TABLE features ADD COLUMN user_uuid TEXT",
                (),
            )
            .await
            .unwrap();
        current_version = fin(current_version, transaction, "add features table").await;
    }

    info!("database migration complete. version: {}", current_version);
}

async fn fin(version: u64, transaction: Transaction, message: &str) -> u64 {
    transaction
        .execute(
            "INSERT INTO migrations(changeset) VALUES (?)",
            params![message],
        )
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    info!("completed migration {}: {}", version, message);

    return version + 1;
}

async fn get_db_version(conn: &Connection) -> u64 {
    log::info!("checking migration version");
    let result = conn
        .query(
            "SELECT version FROM migrations order by version desc limit 1",
            (),
        )
        .await;

    return match result {
        Ok(mut rows) => {
            let first_row = rows.next().await.unwrap();
            let version = first_row
                .map(|row| row.get::<u64>(0).unwrap())
                .unwrap_or_default();

            log::info!("migration version is {}", version);
            version
        }
        Err(_) => {
            log::info!("check failed");
            0
        }
    };
}

pub async fn is_feature_enabled(db: &Connection, feature: &str) -> bool {
    log::info!("checking feature {}", feature);
    let result = db
        .query(
            "SELECT enabled FROM features WHERE name = ?",
            params![feature],
        )
        .await;
    match result {
        Ok(mut rows) => {
            let first_row = rows.next().await.unwrap();
            let enabled = first_row
                .map(|row| row.get::<u64>(0).unwrap())
                .unwrap_or_default();
            log::info!("feature {} is enabled: {}", feature, enabled);
            enabled == 1
        }
        Err(_) => {
            log::info!("checking feature {} failed", feature);
            false
        }
    }
}

pub async fn is_user_feature_enabled(db: &Connection, feature: &str, user_uuid: &str) -> bool {
    log::info!("checking user feature, user: {}, feature: {}", user_uuid, feature);
    let result = db
        .query(
            "SELECT enabled FROM features WHERE name = ? AND user_uuid = ?",
            params![feature, user_uuid],
        )
        .await;
    log::info!("SQL: SELECT enabled FROM features WHERE name = {} AND user_uuid = {}", feature, user_uuid);
    match result {
        Ok(mut rows) => {
            let first_row = rows.next().await.unwrap();
            let enabled = first_row
                .map(|row| row.get::<u64>(0).unwrap())
                .unwrap_or_default();
            log::info!("feature {} is enabled: {}", feature, enabled);
            enabled == 1
        }
        Err(_) => {
            log::info!("checking feature {} failed", feature);
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    filter: String,
    services: Vec<String>,
    label: String,
}
pub async fn log_filters(conn: &Connection) -> Vec<LogFilter> {
    log::info!("getting log filters");
    let result = conn
        .query("SELECT filter, services, label FROM log_filters;", ())
        .await;
    match result {
        Ok(mut rows) => {
            let mut filters = Vec::new();
            while let Ok(row) = rows.next().await {
                match row {
                    Some(row) => {
                        let filter = row.get::<String>(0).unwrap();
                        let services = row
                            .get::<String>(1)
                            .unwrap()
                            .split(";")
                            .map(|s| s.to_string())
                            .collect();
                        let label = row.get::<String>(2).unwrap();
                        filters.push(LogFilter {
                            filter,
                            services,
                            label,
                        })
                    }
                    None => {
                        break;
                    }
                }
            }
            log::info!("log filters {:?}", filters);
            filters
        }
        Err(_) => {
            log::error!("getting log filters failed");
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JepsenConfig {
    pub from_app: String,
    pub to_app: String,
    pub env: String,
    pub auth_api: String,
    pub api_path: String,
    pub api_name: String,
    pub client_id: String,
    pub secret_name: String,
}

pub async fn jepsen_configs(conn: &Connection) -> Vec<JepsenConfig> {
    log::info!("getting jepsen configs ");
    let result = conn
        .query("SELECT from_app, to_app, env, auth_api, api_path, api_name, client_id, secret_name FROM jepsen_configs", ())
        .await;
    match result {
        Ok(mut rows) => {
            let mut configs = Vec::new();
            while let Ok(row) = rows.next().await {
                match row {
                    Some(row) => {
                        let from_app = row.get::<String>(0).unwrap();
                        let to_app = row.get::<String>(1).unwrap();
                        let env = row.get::<String>(2).unwrap();
                        let auth_api = row.get::<String>(3).unwrap();
                        let api_path = row.get::<String>(4).unwrap();
                        let api_name = row.get::<String>(5).unwrap();
                        let client_id = row.get::<String>(6).unwrap();
                        let secret_name = row.get::<String>(7).unwrap();
                        configs.push(JepsenConfig {
                            from_app,
                            to_app,
                            env,
                            auth_api,
                            api_path,
                            api_name,
                            client_id,
                            secret_name,
                        })
                    }
                    None => {
                        break;
                    }
                }
            }
            log::info!("log filters {:?}", &configs);
            configs
        }
        Err(_) => {
            log::error!("getting log filters failed");
            Vec::new()
        }
    }
}
