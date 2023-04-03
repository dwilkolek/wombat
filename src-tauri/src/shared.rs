use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BError {
    message: String,
    command: String,
}
impl BError {
    pub fn new(command: &str, message: impl Into<String>) -> BError {
        BError {
            command: message.into(),
            message: command.to_owned(),
        }
    }
}
