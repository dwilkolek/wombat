use crate::shared::{CommandError, Env, TrackedName};
use log::{error, info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use tracing_unwrap::OptionExt;
use uuid::Uuid;

const RDS_PORT_RANGE: Range<u16> = 52000..52100;
const ECS_PORT_RANGE: Range<u16> = 53000..53100;
const LAMBDA_PORT_RANGE: Range<u16> = 54000..54100;
const COOKIE_SESSION_PORT_RANGE: Range<u16> = 55000..55100;
const FALLBACK_PORT_RANGE: Range<u16> = 58000..59000;

pub fn wombat_dir() -> PathBuf {
    home::home_dir().unwrap_or_log().as_path().join(".wombat")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WombatAwsProfilePreferences {
    pub tracked_names: HashSet<TrackedName>,
    pub preferred_environments: Vec<Env>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub id: Uuid,
    version: i8,
    last_used_profile: Option<String>,
    arn_to_proxy_port_map: Option<HashMap<String, u16>>,
    pub dbeaver_path: Option<String>,
    pub logs_dir: PathBuf,
    pub preferences: HashMap<String, WombatAwsProfilePreferences>,
}

impl UserConfig {
    pub fn default() -> UserConfig {
        let config_file_latest = UserConfig::config_path_latest();
        let config_file_v4 = UserConfig::config_path_v4();
        let config_file_v3 = UserConfig::config_path_v3();
        if config_file_v3.exists() {
            let v3_remove_result = fs::remove_file(config_file_v3);
            match v3_remove_result {
                Ok(_) => info!("Deleted v3 user config"),
                Err(msg) => warn!("Failed to delete v3 user config, reason={}", msg),
            }
        }

        if !config_file_latest.exists() && config_file_v4.exists() {
            match fs::copy(config_file_v4, &config_file_latest) {
                Ok(_) => info!("created copy of v4 config file"),
                Err(e) => error!(
                    "failed to create copy of v4 config file to use as base for v5, reason: {e}"
                ),
            };

            if let Ok(content) = std::fs::read_to_string(&config_file_latest) {
                let updated_content = content
                    .replace("\"preffered_environments\"", "\"preferred_environments\"")
                    .replace("\"verson\"", "\"version\"");
                match std::fs::write(&config_file_latest, updated_content) {
                    Ok(_) => info!("updated v5 contents"),
                    Err(e) => warn!("failed to update v5 contents, reason: {e}"),
                }
            };
        }

        let mut user_config = match std::fs::read_to_string(&config_file_latest) {
            Ok(json) => serde_json::from_str::<UserConfig>(&json).unwrap(),
            Err(_) => UserConfig {
                id: Uuid::new_v4(),
                version: 1,
                last_used_profile: None,
                arn_to_proxy_port_map: Some(HashMap::new()),
                dbeaver_path: None,
                logs_dir: UserConfig::logs_path(),
                preferences: HashMap::new(),
            },
        };

        if user_config.dbeaver_path.is_none() {
            user_config.dbeaver_path =
                UserConfig::recheck_dbeaver_path(user_config.dbeaver_path.clone());
        }

        if user_config.arn_to_proxy_port_map.is_none() {
            user_config.arn_to_proxy_port_map = Some(HashMap::new());
        }

        user_config
    }
    fn config_path_v3() -> PathBuf {
        wombat_dir().join("config.json")
    }
    fn config_path_v4() -> PathBuf {
        wombat_dir().join("config-v4.json")
    }
    fn config_path_latest() -> PathBuf {
        wombat_dir().join("config-v5.json")
    }

    fn logs_path() -> PathBuf {
        wombat_dir().join("log_dumps")
    }

    fn recheck_dbeaver_path(original_path: Option<String>) -> Option<String> {
        if let Some(path) = original_path {
            if std::path::Path::new(&path).exists() {
                return Some(path.to_owned());
            }
        }
        if std::path::Path::new(r"/Applications/DBeaver.app/Contents/MacOS/dbeaver").exists() {
            return Some(r"/Applications/DBeaver.app/Contents/MacOS/dbeaver".to_owned());
        }
        if std::path::Path::new(r"C:\Program Files\DBeaver\dbeaver.exe").exists() {
            return Some(r"C:\Program Files\DBeaver\dbeaver.exe".to_owned());
        }

        None
    }

    pub fn save_preferred_envs(
        &mut self,
        profile_name: &str,
        envs: Vec<Env>,
    ) -> Result<UserConfig, CommandError> {
        let preferences = &mut self.preferences;
        let preference = preferences.get_mut(profile_name).unwrap_or_log();
        preference.preferred_environments = envs;
        self.save();
        Ok(self.clone())
    }

    fn get_port(map: &mut HashMap<String, u16>, arn: &str, range: Range<u16>) -> (u16, bool) {
        if let Some(port) = map.get(arn) {
            return (*port, false);
        }

        let mut rng = rand::rng();
        let range_dist = range.end - range.start;
        let rotate_count = rng.random_range(0..range_dist);

        let used_ports: HashSet<u16> = map.values().copied().collect();
        let next_assignable_port = range
            .into_iter()
            .cycle()
            .skip(rotate_count.into())
            .take(range_dist.into())
            .find(|r| !used_ports.contains(r));

        let assigned_port = match next_assignable_port {
            Some(port) => port,
            None => FALLBACK_PORT_RANGE
                .into_iter()
                .find(|p| !used_ports.contains(p))
                .expect("No available ports left!"),
        };

        map.insert(arn.to_owned(), assigned_port);
        (assigned_port, true)
    }

    pub fn get_db_port(&mut self, db_arn: &str) -> u16 {
        let port = Self::get_port(
            self.arn_to_proxy_port_map.as_mut().unwrap_or_log(),
            db_arn,
            RDS_PORT_RANGE,
        );
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_service_port(&mut self, ecs_arn: &str) -> u16 {
        let port = Self::get_port(
            self.arn_to_proxy_port_map.as_mut().unwrap_or_log(),
            ecs_arn,
            ECS_PORT_RANGE,
        );
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_lambda_app_port(&mut self, lambda_arn: &str) -> u16 {
        let port = Self::get_port(
            self.arn_to_proxy_port_map.as_mut().unwrap_or_log(),
            lambda_arn,
            LAMBDA_PORT_RANGE,
        );
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_user_session_proxy_port(&mut self, user_session_arn: &str) -> u16 {
        let port = Self::get_port(
            self.arn_to_proxy_port_map.as_mut().unwrap_or_log(),
            user_session_arn,
            COOKIE_SESSION_PORT_RANGE,
        );
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn set_dbeaver_path(&mut self, dbeaver_path: &str) -> Result<UserConfig, CommandError> {
        if std::path::Path::new(dbeaver_path).exists() {
            self.dbeaver_path = Some(dbeaver_path.to_owned());
            self.save();
            Ok(self.clone())
        } else {
            Err(CommandError::new("set_dbeaver_path", "Invalid path!"))
        }
    }
    pub fn set_logs_path(&mut self, logs_dir_path: &str) -> Result<UserConfig, CommandError> {
        let path = std::path::Path::new(logs_dir_path);
        let res = fs::create_dir_all(path);
        match res {
            Err(msg) => Err(CommandError::new(
                "set_logs_path",
                format!("Invalid path! {msg}"),
            )),
            Ok(()) => {
                self.logs_dir = PathBuf::from(logs_dir_path.to_owned());
                self.save();
                Ok(self.clone())
            }
        }
    }

    pub fn use_profile(&mut self, profile: &str, envs: Vec<Env>, tracked_names: HashSet<String>) {
        info!("Using profile: {profile}, envs={envs:?}, tracked_names={tracked_names:?}");
        self.last_used_profile = Some(profile.to_owned());
        let preferences = &mut self.preferences;
        if !preferences.contains_key(profile) {
            preferences.insert(
                profile.to_owned(),
                WombatAwsProfilePreferences {
                    tracked_names,
                    preferred_environments: envs,
                },
            );
        }
        self.save()
    }

    pub fn favorite(
        &mut self,
        profile_name: &str,
        tracked_name: TrackedName,
    ) -> Result<UserConfig, CommandError> {
        info!("Favorite {} ", &tracked_name);
        let preferences = &mut self.preferences;
        let preference = preferences.get_mut(profile_name).unwrap_or_log();
        if !preference.tracked_names.remove(&tracked_name) {
            info!("Favorite Add {} ", &tracked_name);
            preference.tracked_names.insert(tracked_name);
        }
        self.save();
        Ok(self.clone())
    }

    fn save(&self) {
        info!("Storing to: {:?}", UserConfig::config_path_latest());
        std::fs::write(
            UserConfig::config_path_latest(),
            serde_json::to_string_pretty(self).expect("Failed to serialize user config"),
        )
        .expect("Failed to save user config");
    }
}
