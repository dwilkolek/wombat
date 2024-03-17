use std::{collections::HashMap, process::Command};

pub fn check_dependencies() -> HashMap<String, Result<String, String>> {
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

    return dependecies;
}
