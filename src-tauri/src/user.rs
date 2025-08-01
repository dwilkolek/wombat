use log::{error, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tracing_unwrap::OptionExt;

use crate::shared::{arn_to_name, CommandError, Env, TrackedName};
use uuid::Uuid;

pub fn wombat_dir() -> PathBuf {
    home::home_dir().unwrap_or_log().as_path().join(".wombat")
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WombatAwsProfilePreferences {
    pub tracked_names: HashSet<TrackedName>,
    pub preffered_environments: Vec<Env>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub id: Uuid,
    verson: i8,
    last_used_profile: Option<String>,
    db_proxy_port_map: HashMap<TrackedName, HashMap<Env, u16>>,
    service_proxy_port_map: HashMap<TrackedName, HashMap<Env, u16>>,
    lambda_app_proxy_port_map: Option<HashMap<TrackedName, HashMap<Env, u16>>>,
    user_session_proxy_port_map: Option<HashMap<String, u16>>,
    pub dbeaver_path: Option<String>,
    pub logs_dir: Option<PathBuf>,
    pub preferences: Option<HashMap<String, WombatAwsProfilePreferences>>,
}

impl UserConfig {
    pub fn default() -> UserConfig {
        let config_file = UserConfig::config_path();
        let config_file_v3 = UserConfig::config_path_v3();
        if !config_file.exists() && config_file_v3.exists() {
            match fs::copy(config_file_v3, &config_file) {
                Ok(_) => info!("created copy of v3 config file"),
                Err(e) => error!(
                    "failed to create copy of v3 config file to use as base for v4, reason: {e}"
                ),
            };
        }

        let mut user_config = match std::fs::read_to_string(config_file) {
            Ok(json) => serde_json::from_str::<UserConfig>(&json).unwrap(),
            Err(_) => UserConfig {
                id: Uuid::new_v4(),
                verson: 1,
                last_used_profile: None,

                db_proxy_port_map: HashMap::new(),
                service_proxy_port_map: HashMap::new(),
                lambda_app_proxy_port_map: Some(HashMap::new()),
                user_session_proxy_port_map: Some(HashMap::new()),

                dbeaver_path: None,
                logs_dir: Some(UserConfig::logs_path()),
                preferences: Some(HashMap::new()),
            },
        };

        if user_config.dbeaver_path.is_none() {
            user_config.dbeaver_path =
                UserConfig::recheck_dbeaver_path(user_config.dbeaver_path.clone());
        }
        if user_config.logs_dir.is_none() {
            user_config.logs_dir = Some(UserConfig::logs_path());
        }
        if user_config.preferences.is_none() {
            user_config.preferences = Some(HashMap::new());
        }
        if user_config.lambda_app_proxy_port_map.is_none() {
            user_config.lambda_app_proxy_port_map = Some(HashMap::new());
        }
        if user_config.user_session_proxy_port_map.is_none() {
            user_config.user_session_proxy_port_map = Some(HashMap::new());
        }

        user_config
    }
    fn config_path_v3() -> PathBuf {
        wombat_dir().join("config.json")
    }
    fn config_path() -> PathBuf {
        wombat_dir().join("config-v4.json")
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

    pub fn save_preffered_envs(
        &mut self,
        profile_name: &str,
        envs: Vec<Env>,
    ) -> Result<UserConfig, CommandError> {
        let preferences = self.preferences.as_mut().unwrap_or_log();
        let preference = preferences.get_mut(profile_name).unwrap_or_log();
        preference.preffered_environments = envs;
        self.save();
        Ok(self.clone())
    }

    fn get_port(
        map: &mut HashMap<TrackedName, HashMap<Env, u16>>,
        tracked_name: TrackedName,
        env: Env,
        from_port: u16,
        range: u16,
    ) -> (u16, bool) {
        if let Some(mapping) = map.get(&tracked_name) {
            if let Some(port) = mapping.get(&env) {
                return (*port, false);
            }
        }

        let used_ports: Vec<u16> = map.values().flat_map(|e| e.values()).copied().collect();

        let mut possible_port = rand::rng().random_range(from_port..from_port + range);
        while used_ports.contains(&possible_port) {
            possible_port = rand::rng().random_range(from_port..from_port + range);
        }
        if !map.contains_key(&tracked_name) {
            map.insert(tracked_name.clone(), HashMap::new());
        }
        if let Some(mapping) = map.get_mut(&tracked_name) {
            mapping.insert(env, possible_port);
        }

        (possible_port, true)
    }

    pub fn get_db_port(&mut self, db_arn: &str) -> u16 {
        let env = Env::from_any(db_arn);
        let tracked_name = arn_to_name(db_arn);
        let port = Self::get_port(&mut self.db_proxy_port_map, tracked_name, env, 52000, 100);
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_service_port(&mut self, ecs_arn: &str) -> u16 {
        let env = Env::from_any(ecs_arn);
        let tracked_name = arn_to_name(ecs_arn);
        let port = Self::get_port(
            &mut self.service_proxy_port_map,
            tracked_name,
            env,
            53000,
            100,
        );
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_lambda_app_port(&mut self, app: &str, env: &Env) -> u16 {
        let port = Self::get_port(
            self.lambda_app_proxy_port_map.as_mut().unwrap(),
            app.to_owned(),
            env.to_owned(),
            54000,
            100,
        );
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_user_session_proxy_port(&mut self, address: &str) -> u16 {
        let from_port = 55000;
        let range = 100;
        let map = self.user_session_proxy_port_map.as_mut().unwrap();
        if let Some(port) = map.get(address) {
            return *port;
        }

        let used_ports: Vec<u16> = map.values().copied().collect();

        let mut possible_port = rand::rng().random_range(from_port..from_port + range);
        while used_ports.contains(&possible_port) {
            possible_port = rand::rng().random_range(from_port..from_port + range);
        }

        map.insert(address.to_owned(), possible_port);
        self.save();

        possible_port
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
                self.logs_dir = Some(PathBuf::from(logs_dir_path.to_owned()));
                self.save();
                Ok(self.clone())
            }
        }
    }

    pub fn use_profile(&mut self, profile: &str, envs: Vec<Env>, tracked_names: HashSet<String>) {
        info!("Using profile: {profile}, envs={envs:?}, tracked_names={tracked_names:?}");
        self.last_used_profile = Some(profile.to_owned());
        let preferences = self.preferences.as_mut().unwrap_or_log();
        if !preferences.contains_key(profile) {
            preferences.insert(
                profile.to_owned(),
                WombatAwsProfilePreferences {
                    tracked_names,
                    preffered_environments: envs,
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
        let preferences = self.preferences.as_mut().unwrap_or_log();
        let preference = preferences.get_mut(profile_name).unwrap_or_log();
        if !preference.tracked_names.remove(&tracked_name) {
            info!("Favorite Add {} ", &tracked_name);
            preference.tracked_names.insert(tracked_name);
        }
        self.save();
        Ok(self.clone())
    }

    fn save(&self) {
        info!("Storing to: {:?}", UserConfig::config_path());
        std::fs::write(
            UserConfig::config_path(),
            serde_json::to_string_pretty(self).expect("Failed to serialize user config"),
        )
        .expect("Failed to save user config");
    }
}
