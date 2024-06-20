use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use crate::{aws, wombat_api::WombatApi};

pub async fn check_dependencies(
    wombat_api: &mut WombatApi,
    aws_config_provider: &aws::AwsConfigProvider,
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
    dependecies.insert(
        "aws-profiles".to_string(),
        Ok(format!(
            "wombat: {}, sso: {}, infra: {}",
            &aws_config_provider.wombat_profiles.len(),
            &aws_config_provider
                .wombat_profiles
                .iter()
                .map(|w| w.sso_profiles.len())
                .sum::<usize>(),
            &aws_config_provider
                .wombat_profiles
                .iter()
                .flat_map(|w| w.sso_profiles.iter().map(|sso| {
                    sso.1
                        .infra_profiles
                        .iter()
                        .map(|infra| infra.profile_name.as_ref())
                        .collect::<HashSet<&str>>()
                        .len()
                }))
                .sum::<usize>()
        )),
    );

    let wombat_api_key = "wombat-backend-api".to_string();

    dependecies.insert(wombat_api_key, wombat_api.status().await);

    dependecies
}
