use std::{collections::HashSet, env, fs, process::Command};

use crate::{aws, wombat_api::WombatApi};

pub const CODEARTIFACT_LOGIN: &str = "codeartifact-login";
pub const AWS_CLI: &str = "aws-cli";
pub const SESSION_MANAGER_PLUGIN: &str = "session-manager-plugin";
pub const WOMBAT_API: &str = "wombat-backend-api";

#[derive(Clone, serde::Serialize)]
pub struct Dependency {
    name: String,
    ok: bool,
    required: bool,
    version_or_error: String,
}

pub async fn check_dependencies(
    wombat_api: &mut WombatApi,
    aws_config_provider: &aws::AwsConfigProvider,
    required_feature: &str,
) -> Vec<Dependency> {
    let mut dependecies = Vec::new();
    log::info!("checking dependencies");
    if let Ok(cmd) = Command::new("aws").arg("--version").output() {
        if cmd.status.success() {
            let version = String::from_utf8_lossy(&cmd.stdout).to_string();
            dependecies.push(Dependency {
                name: AWS_CLI.to_string(),
                ok: true,
                required: true,
                version_or_error: version,
            });
        } else {
            let error = String::from_utf8_lossy(&cmd.stderr).to_string();
            dependecies.push(Dependency {
                name: AWS_CLI.to_string(),
                ok: false,
                required: true,
                version_or_error: error,
            });
        }
    } else {
        dependecies.push(Dependency {
            name: AWS_CLI.to_string(),
            ok: false,
            required: true,
            version_or_error: "Not installed".to_string(),
        });
    }

    log::info!("aws done");
    if let Ok(cmd) = Command::new(SESSION_MANAGER_PLUGIN)
        .arg("--version")
        .output()
    {
        if cmd.status.success() {
            let version = String::from_utf8_lossy(&cmd.stdout).to_string();
            dependecies.push(Dependency {
                name: SESSION_MANAGER_PLUGIN.to_string(),
                ok: true,
                required: true,
                version_or_error: version,
            });
        } else {
            let error = String::from_utf8_lossy(&cmd.stderr).to_string();
            dependecies.push(Dependency {
                name: SESSION_MANAGER_PLUGIN.to_string(),
                ok: true,
                required: true,
                version_or_error: error,
            });
        }
    } else {
        dependecies.push(Dependency {
            name: SESSION_MANAGER_PLUGIN.to_string(),
            ok: true,
            required: true,
            version_or_error: "Not installed".to_string(),
        });
    }
    log::info!("session manager done");
    let wombat_profiles = &aws_config_provider.wombat_profiles.len();
    let sso_profile = &aws_config_provider
        .wombat_profiles
        .iter()
        .map(|w| w.sso_profiles.len())
        .sum::<usize>();
    let infra_profiles = &aws_config_provider
        .wombat_profiles
        .iter()
        .flat_map(|w| {
            w.sso_profiles.iter().map(|sso| {
                sso.1
                    .infra_profiles
                    .iter()
                    .map(|infra| infra.profile_name.as_ref())
                    .collect::<HashSet<&str>>()
                    .len()
            })
        })
        .sum::<usize>();
    dependecies.push(Dependency {
        name: "aws-profiles".to_string(),
        ok: *sso_profile > 0,
        required: true,
        version_or_error: format!(
            "wombat: {}, sso: {}, infra: {}",
            wombat_profiles, sso_profile, infra_profiles
        ),
    });

    log::info!("wombat done");
    let wombat_api_state = wombat_api.status(required_feature).await;
    dependecies.push(match wombat_api_state {
        Ok(v) => Dependency {
            name: WOMBAT_API.to_string(),
            ok: true,
            required: true,
            version_or_error: v,
        },
        Err(v) => Dependency {
            name: WOMBAT_API.to_string(),
            ok: false,
            required: true,
            version_or_error: v,
        },
    });

    let is_codeartifact_login_in_path = is_program_in_path(CODEARTIFACT_LOGIN);
    dependecies.push(Dependency {
        name: CODEARTIFACT_LOGIN.to_string(),
        ok: is_codeartifact_login_in_path,
        required: false,
        version_or_error: if is_codeartifact_login_in_path {
            "Exists in PATH".to_string()
        } else {
            "Not in PATH".to_string()
        },
    });

    dependecies
}

pub fn is_program_in_path(program: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(":") {
            let p_str = format!("{}/{}", p, program);
            if fs::metadata(p_str).is_ok() {
                return true;
            }
        }
    }
    false
}
