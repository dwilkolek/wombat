use chrono::{DateTime, Utc};
use core::fmt;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing_unwrap::{OptionExt, ResultExt};

pub type TrackedName = String;

pub struct BrowserExtension {
    pub version: Option<String>,
    pub reported_version: Option<String>,
    pub last_health_check: DateTime<Utc>,
    pub expected_version: String,
    pub last_supported_version: String,
}
impl BrowserExtension {
    pub fn to_status(&self) -> BrowserExtensionStatus {
        let version = self.version.clone().unwrap_or("0.0.0".to_owned());
        let numbered_version = version_to_number(&version);
        let state = if (Utc::now() - self.last_health_check).num_seconds() < 10 {
            if numbered_version >= version_to_number(&self.expected_version) {
                BrowserExtensionState::UpToDate
            } else if numbered_version < version_to_number(&self.last_supported_version) {
                BrowserExtensionState::NotSupported
            } else {
                BrowserExtensionState::Outdated
            }
        } else {
            BrowserExtensionState::Disconnected
        };
        BrowserExtensionStatus {
            state,
            version: self.version.clone(),
        }
    }
}

fn version_to_number(version: &str) -> u32 {
    version
        .split('.')
        .enumerate()
        .map(|(i, v)| {
            v.parse::<u32>()
                .map(|num| num * 1000_u32.pow((2 - i).try_into().unwrap()))
                .unwrap_or(0)
        })
        .sum()
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub env: Env,
    pub stored_at: DateTime<Utc>,
}

pub struct CookieJar {
    pub cookies: Vec<Cookie>,
}

impl CookieJar {
    pub fn header_value_for_env(&self, env: &Env) -> String {
        self.cookies
            .iter()
            .filter(|c| c.env == *env)
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<String>>()
            .join("; ")
    }
    pub fn to_status(&self) -> CookieJarStatus {
        CookieJarStatus {
            cookie_health: self
                .cookies
                .iter()
                .map(|cookie| {
                    let cookie_health = match (Utc::now() - cookie.stored_at).num_seconds() {
                        0..=300 => CookieHealth::Ok,
                        301..=600 => CookieHealth::Stale,
                        _ => CookieHealth::Old,
                    };

                    (cookie.env.clone(), cookie_health)
                })
                .collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CookieJarStatus {
    pub cookie_health: HashMap<Env, CookieHealth>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum BrowserExtensionState {
    Disconnected,
    NotSupported,
    Outdated,
    UpToDate,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BrowserExtensionStatus {
    pub state: BrowserExtensionState,
    pub version: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum CookieHealth {
    Ok,
    Stale,
    Old,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ResourceType {
    RDS,
    ECS,
    LambdaApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    pub message: String,
    pub command: String,
}
impl CommandError {
    pub fn new(command: &str, message: impl Into<String>) -> CommandError {
        CommandError {
            command: command.to_owned(),
            message: message.into(),
        }
    }
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

fn ecs_arn_to_name(arn: &str) -> TrackedName {
    arn.split('/').next_back().unwrap_or_log().to_owned()
}

fn rds_arn_to_name(arn: &str) -> TrackedName {
    arn.split(':')
        .next_back()
        .unwrap_or_log()
        .split('-')
        .filter(|part| part != &"dsi" && !(["play", "lab", "dev", "demo", "prod"].contains(part)))
        .collect::<Vec<&str>>()
        .join("-")
}

pub fn cluster_arn_to_name(arn: &str) -> TrackedName {
    arn.split('/').next_back().unwrap_or_log().to_owned()
}

pub fn arn_to_name(arn: &str) -> TrackedName {
    if arn.starts_with("arn:aws:ecs") {
        return ecs_arn_to_name(arn);
    }
    if arn.starts_with("arn:aws:rds") {
        return rds_arn_to_name(arn);
    }
    if arn.starts_with("lambdaApp::") {
        return arn.split("::").skip(1).take(1).collect();
    }
    format!("unknown!#{}", arn)
}
