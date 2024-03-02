use std::time::Duration;

use libsql::{params, Connection};
use serde::{Deserialize, Serialize};

pub struct GlobDatabase {
    db: std::sync::Arc<tokio::sync::RwLock<libsql::Database>>,
    pub synchronized: bool,
}

impl GlobDatabase {
    pub fn new(db: libsql::Database) -> Self {
        GlobDatabase {
            db: std::sync::Arc::new(tokio::sync::RwLock::new(db)),
            synchronized: false,
        }
    }

    pub async fn sync(&mut self) {
        self.synchronized = true;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let db = self.db.clone();
        let handler = tokio::task::spawn(async move {
            let db = db.read().await;
            let sync_result = db.sync().await;
            log::info!("sync result: {:?}", sync_result);

            tx.send(()).unwrap();
        });
        if let Err(_) = tokio::time::timeout(Duration::from_millis(10000), rx).await {
            println!("did not receive value within 10s");
            self.synchronized = false;
            handler.abort();
        }
    }

    pub async fn get_connection(&self) -> Connection {
        let db = self.db.read().await;
        db.connect().unwrap()
    }
}

pub async fn is_feature_enabled(db: &GlobDatabase, feature: &str) -> bool {
    log::info!("checking feature {}", feature);
    let conn = db.get_connection().await;
    let result = conn
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

pub async fn is_user_feature_enabled(db: &GlobDatabase, feature: &str, user_uuid: &str) -> bool {
    if db.synchronized == false {
        return false;
    }
    let conn = db.get_connection().await;
    log::info!(
        "checking user feature, user: {}, feature: {}",
        user_uuid,
        feature
    );
    let result = conn
        .query(
            "SELECT enabled FROM features WHERE name = ? AND user_uuid = ?",
            params![feature, user_uuid],
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
pub async fn log_filters(db: &GlobDatabase) -> Vec<LogFilter> {
    log::info!("getting log filters");

    if db.synchronized == false {
        return vec![];
    }
    let conn = db.get_connection().await;

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
            log::info!("log filters total: {}", filters.len());
            filters
        }
        Err(e) => {
            log::error!("getting log filters failed, {}", e);
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuthConfig {
    pub to_app: String,
    pub env: String,

    pub auth_type: String,
    pub api_path: String,

    pub jepsen_auth_api: Option<String>,
    pub jepsen_api_name: Option<String>,
    pub jepsen_client_id: Option<String>,

    pub basic_user: Option<String>,

    pub secret_name: String,
}

pub async fn get_proxy_auth_configs(db: &GlobDatabase) -> Vec<ProxyAuthConfig> {
    log::info!("getting proxy auth configs");
    if db.synchronized == false {
        return vec![];
    }
    let conn = db.get_connection().await;
    let result = conn
        .query(
            "SELECT to_app, env,
            auth_type, api_path,

            jepsen_auth_api,
            jepsen_api_name,
            jepsen_client_id,

            basic_user,

            secret_name
        
         FROM proxy_auth_configs",
            (),
        )
        .await;
    match result {
        Ok(mut rows) => {
            let mut configs = Vec::new();
            while let Ok(row) = rows.next().await {
                match row {
                    Some(row) => match libsql::de::from_row::<ProxyAuthConfig>(&row) {
                        Ok(auth) => configs.push(auth),
                        Err(e) => log::error!("failed to parse proxy auth config: {}", e),
                    },
                    None => {
                        break;
                    }
                }
            }
            log::info!("proxy auth configs, total: {}", configs.len());
            configs
        }
        Err(e) => {
            log::error!("getting proxy auth configs failed, {}", e);
            Vec::new()
        }
    }
}
