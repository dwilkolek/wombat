
use core::fmt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use chrono::prelude::*;
use tracing_unwrap::ResultExt;




#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bastion {
    pub instance_id: String,
    pub env: Env,
}



#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Env {
    DEVNULL,
    PLAY,
    LAB,
    DEV,
    DEMO,
    PROD,
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Env::DEVNULL => write!(f, "devnull"),
            Env::PLAY => write!(f, "play"),
            Env::LAB => write!(f, "lab"),
            Env::DEV => write!(f, "dev"),
            Env::DEMO => write!(f, "demo"),
            Env::PROD => write!(f, "prod"),
        }
    }
}

impl Env {
    pub fn from_exact(str: &str) -> Env {
        match str {
            "play" => Env::PLAY,
            "lab" => Env::LAB,
            "dev" => Env::DEV,
            "demo" => Env::DEMO,
            "prod" => Env::PROD,
            _ => Env::DEVNULL,
        }
    }
    pub fn from_any(str: &str) -> Env {
        let env_regex = Regex::new(".*(play|lab|dev|demo|prod).*").unwrap_or_log();
        let captures = env_regex.captures(str);
        let env = captures
            .and_then(|c| c.get(1))
            .map(|e| e.as_str().to_owned())
            .unwrap_or("".to_owned());

        Env::from_exact(&env)
    }
}



#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Cluster {
    pub name: String,
    pub arn: String,
    pub env: Env,
    pub platform_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdsInstance {
    pub arn: String,
    pub name: String,
    pub normalized_name: String,
    pub engine: String,
    pub engine_version: String,
    pub endpoint: Endpoint,
    pub environment_tag: String,
    pub env: Env,
    pub appname_tag: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct DbSecretDTO {
    dbInstanceIdentifier: String,
    pub dbname: String,
    engine: String,
    host: String,
    pub password: String,
    port: u16,
    pub username: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DbSecret {
    pub dbname: String,
    pub password: String,
    pub username: String,
    pub auto_rotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsService {
    pub arn: String,
    pub name: String,
    pub cluster_arn: String,
    pub env: Env,
    pub td_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetails {
    pub timestamp: DateTime<Utc>,
    pub arn: String,
    pub name: String,
    pub version: String,
    pub cluster_arn: String,
    pub env: Env,
    pub task_registered_at: Option<DateTime<Utc>>,
    pub td_family: String,
    pub td_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetailsMissing {
    pub timestamp: DateTime<Utc>,
    pub arn: String,
    pub name: String,
    pub error: String,
    pub env: Env,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub log_stream_name: String,
    pub timestamp: i64,
    pub ingestion_time: i64,
    pub message: String,
}
