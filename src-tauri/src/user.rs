use log::info;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use tracing_unwrap::OptionExt;

use crate::shared::{ecs_arn_to_name, rds_arn_to_name, BError, Env, TrackedName};
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
    pub dbeaver_path: Option<String>,
    pub logs_dir: Option<PathBuf>,
    pub preferences: Option<HashMap<String, WombatAwsProfilePreferences>>
}

impl UserConfig {
    pub fn default() -> UserConfig {
        let config_file = UserConfig::config_path();
        let mut restored = false;
        if !config_file.exists() {
            let _ = fs::create_dir_all(wombat_dir());
            let old_config = home::home_dir()
                .unwrap_or_log()
                .as_path()
                .join(".wombat_v1");

            if old_config.exists() {
                info!("Migrating old config");
                restored = true;
                let _ = fs::copy(&old_config, &config_file);
                let _ = fs::remove_file(&old_config);
            }
        }

        let mut user_config = match std::fs::read_to_string(config_file) {
            Ok(json) => serde_json::from_str::<UserConfig>(&json).unwrap(),
            Err(_) => UserConfig {
                id: Uuid::new_v4(),
                verson: 1,
                last_used_profile: None,
                
                db_proxy_port_map: HashMap::new(),
                service_proxy_port_map: HashMap::new(),
                dbeaver_path: None,
                logs_dir: Some(UserConfig::logs_path()),
                preferences: Some(HashMap::new())
            },
        };

        if user_config.dbeaver_path.is_none() {
            user_config.dbeaver_path =
                UserConfig::recheck_dbeaver_path(user_config.dbeaver_path.clone());
        }
        if user_config.logs_dir.is_none() || restored {
            user_config.logs_dir = Some(UserConfig::logs_path());
        } 
        if user_config.preferences.is_none(){
            user_config.preferences =Some(HashMap::new());
        }

        user_config
    }

    fn config_path() -> PathBuf {
        wombat_dir().join("config.json")
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

    pub fn save_preffered_envs(&mut self, profile_name: &str, envs: Vec<Env>) -> Result<UserConfig, BError> {
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
    ) -> (u16, bool) {
        if let Some(mapping) = map.get(&tracked_name) {
            if let Some(port) = mapping.get(&env) {
                return (*port, false);
            }
        }

        let used_ports: Vec<u16> = map.values().flat_map(|e| e.values()).copied().collect();

        let mut possible_port = rand::thread_rng().gen_range(52000..53000);
        while used_ports.iter().any(|p| *p == possible_port) {
            possible_port = rand::thread_rng().gen_range(52000..53000);
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
        let tracked_name = rds_arn_to_name(db_arn);
        let port = Self::get_port(&mut self.db_proxy_port_map, tracked_name, env);
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn get_service_port(&mut self, ecs_arn: &str) -> u16 {
        let env = Env::from_any(ecs_arn);
        let tracked_name = ecs_arn_to_name(ecs_arn);
        let port = Self::get_port(&mut self.service_proxy_port_map, tracked_name, env);
        if port.1 {
            self.save()
        }
        port.0
    }

    pub fn set_dbeaver_path(&mut self, dbeaver_path: &str) -> Result<UserConfig, BError> {
        if std::path::Path::new(dbeaver_path).exists() {
            self.dbeaver_path = Some(dbeaver_path.to_owned());
            self.save();
            Ok(self.clone())
        } else {
            Err(BError::new("set_dbeaver_path", "Invalid path!"))
        }
    }
    pub fn set_logs_path(&mut self, logs_dir_path: &str) -> Result<UserConfig, BError> {
        let path = std::path::Path::new(logs_dir_path);
        let res = fs::create_dir_all(path);
        match res {
            Err(msg) => Err(BError::new(
                "set_logs_path",
                format!("Invalid path! {}", msg),
            )),
            Ok(()) => {
                self.logs_dir = Some(PathBuf::from(logs_dir_path.to_owned()));
                self.save();
                Ok(self.clone())
            }
        }
    }

    pub fn use_profile(&mut self, profile: &str, envs: Vec<Env>, tracked_names: HashSet<String>) {
        self.last_used_profile = Some(profile.to_owned());
        let preferences = self.preferences.as_mut().unwrap_or_log();
        if !preferences.contains_key(profile) {
            preferences.insert(profile.to_owned(), WombatAwsProfilePreferences {
                tracked_names: tracked_names,
                preffered_environments: envs
            });
        }
        self.save()
    }

    pub fn favorite(&mut self, profile_name: &str, tracked_name: TrackedName) -> Result<UserConfig, BError> {
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
