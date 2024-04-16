use std::{collections::HashMap, process::Command};

use crate::wombat_api::WombatApi;

pub async fn check_dependencies(
    wombat_api: &mut WombatApi,
) -> HashMap<String, Result<String, String>> {
    let mut dependecies = HashMap::new();

    if let Ok(cmd) = Command::new("aws").arg("--version").output() {
        if cmd.status.success() {
            let version = String::from_utf8_lossy(&cmd.stdout).to_string();
            dependecies.insert("aws-cli".to_string(), Ok(version));
        } else {
            let error = String::from_utf8_lossy(&cmd.stderr).to_string();
            dependecies.insert("aws-cli".to_string(), Err(error));
        }
    } else {
        dependecies.insert("aws-cli".to_string(), Err("Not installed".to_string()));
    }

    if let Ok(cmd) = Command::new("session-manager-plugin")
        .arg("--version")
        .output()
    {
        if cmd.status.success() {
            let version = String::from_utf8_lossy(&cmd.stdout).to_string();
            dependecies.insert("session-manager-plugin".to_string(), Ok(version));
        } else {
            let error = String::from_utf8_lossy(&cmd.stderr).to_string();
            dependecies.insert("session-manager-plugin".to_string(), Err(error));
        }
    } else {
        dependecies.insert(
            "session-manager-plugin".to_string(),
            Err("Not installed".to_string()),
        );
    }

    let wombat_api_key = "wombat-backend-api".to_string();

    dependecies.insert(wombat_api_key, wombat_api.status().await);

    dependecies
}
