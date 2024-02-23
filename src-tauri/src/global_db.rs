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
            .execute("CREATE TABLE features(name TEXT, enabled BOOLEAN)", ())
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
                "CREATE TABLE logFilters(filter TEXT, label TEXT, services TEXT)",
                (),
            )
            .await
            .unwrap();
        current_version = fin(current_version, transaction, "add logs filter table").await;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    filter: String,
    services: Vec<String>,
    label: String,
}
pub async fn log_filters(conn: &Connection) -> Vec<LogFilter> {
    log::info!("getting log filters");
    let result = conn
        .query("SELECT filter, services, label FROM logFilters;", ())
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
