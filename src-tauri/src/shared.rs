use chrono::{DateTime, Utc};
use core::fmt;
use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};
use tracing_unwrap::{OptionExt, ResultExt};

pub type TrackedName = String;

pub struct CookieJar {
    pub cookies: HashMap<String, (DateTime<Utc>, String)>,
    pub last_health_check: DateTime<Utc>,
}

impl CookieJar {
    pub fn to_status(&self) -> BrowserExtensionStatus {
        BrowserExtensionStatus {
            connected: (Utc::now() - self.last_health_check).num_seconds() < 10,
            cookie_health: self
                .cookies
                .iter()
                .map(|entry| {
                    let env = Env::from_any(entry.0);
                    let cookie_health = match (Utc::now() - entry.1 .0).num_seconds() {
                        0..=300 => CookieHealth::Ok,
                        301..=600 => CookieHealth::Stale,
                        _ => CookieHealth::Old,
                    };

                    (env, cookie_health)
                })
                .collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BrowserExtensionStatus {
    pub connected: bool,
    pub cookie_health: HashMap<Env, CookieHealth>,
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
pub struct BError {
    pub message: String,
    pub command: String,
}
impl BError {
    pub fn new(command: &str, message: impl Into<String>) -> BError {
        BError {
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

pub fn ecs_arn_to_name(arn: &str) -> TrackedName {
    arn.split('/').last().unwrap_or_log().to_owned()
}

pub fn rds_arn_to_name(arn: &str) -> TrackedName {
    arn.split(':')
        .last()
        .unwrap_or_log()
        .split('-')
        .filter(|part| part != &"dsi" && !(["play", "lab", "dev", "demo", "prod"].contains(part)))
        .collect::<Vec<&str>>()
        .join("-")
}

pub fn cluster_arn_to_name(arn: &str) -> TrackedName {
    arn.split('/').last().unwrap_or_log().to_owned()
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
pub fn arn_resource_type(arn: &str) -> Option<ResourceType> {
    if arn.starts_with("arn:aws:ecs") {
        return Some(ResourceType::ECS);
    }
    if arn.starts_with("arn:aws:rds") {
        return Some(ResourceType::RDS);
    }
    if arn.starts_with("lambdaApp::") {
        return Some(ResourceType::LambdaApp);
    }
    error!("Unknown resource type given arn {}", arn);
    None
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
