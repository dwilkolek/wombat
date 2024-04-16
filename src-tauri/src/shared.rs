use core::fmt;

use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing_unwrap::{OptionExt, ResultExt};

pub type TrackedName = String;

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ResourceType {
    RDS,
    ECS,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    format!("unknown!#{}", arn)
}
pub fn arn_resource_type(arn: &str) -> Option<ResourceType> {
    if arn.starts_with("arn:aws:ecs") {
        return Some(ResourceType::ECS);
    }
    if arn.starts_with("arn:aws:rds") {
        return Some(ResourceType::RDS);
    }
    error!("Unknown resource type given arn {}", arn);
    None
}
