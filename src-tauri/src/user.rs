use log::info;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::shared::{ecs_arn_to_name, rds_arn_to_name, BError, Env, TrackedName};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfigOld {
    id: Uuid,
    last_used_profile: Option<String>,
    known_profiles: HashSet<String>,
    pub ecs: HashSet<String>,
    pub rds: HashSet<String>,
    db_proxy_port_map: HashMap<String, u16>,
    service_proxy_port_map: HashMap<String, u16>,
    pub dbeaver_path: Option<String>,
}

impl UserConfigOld {
    pub fn default() -> Option<Self> {
        let config_file = Self::config_path();
        match std::fs::read_to_string(config_file) {
            Ok(json) => match serde_json::from_str::<Self>(&json) {
                Ok(profile) => Some(profile),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    fn config_path() -> PathBuf {
        home::home_dir().unwrap().as_path().join(".wombat")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    pub id: Uuid,
    verson: i8,
    last_used_profile: Option<String>,
    known_profiles: HashSet<String>,
    pub tracked_names: HashSet<TrackedName>,
    db_proxy_port_map: HashMap<TrackedName, HashMap<Env, u16>>,
    service_proxy_port_map: HashMap<TrackedName, HashMap<Env, u16>>,
    pub dbeaver_path: Option<String>,
    pub preffered_environments: Vec<Env>,
}

impl UserConfig {
    pub fn default() -> UserConfig {
        let old_config = UserConfigOld::default();
        if let Some(old) = old_config {
            let ecs_tracked_names: HashSet<TrackedName> =
                old.ecs.iter().map(|arn| ecs_arn_to_name(arn)).collect();
            let rds_tracked_names: HashSet<TrackedName> =
                old.rds.iter().map(|arn| rds_arn_to_name(arn)).collect();
            let mut tracked_names: HashSet<TrackedName> = HashSet::new();
            tracked_names.extend(ecs_tracked_names);
            tracked_names.extend(rds_tracked_names);

            let mut db_proxy_port_map: HashMap<TrackedName, HashMap<Env, u16>> = HashMap::new();
            for old_entry in old.db_proxy_port_map.iter() {
                let tracked_name = rds_arn_to_name(old_entry.0);
                let env = Env::from_any(old_entry.0);
                if !db_proxy_port_map.contains_key(&tracked_name) {
                    db_proxy_port_map.insert(tracked_name.clone(), HashMap::new());
                }
                db_proxy_port_map
                    .get_mut(&tracked_name)
                    .unwrap()
                    .insert(env, *old_entry.1);
            }

            let mut service_proxy_port_map: HashMap<TrackedName, HashMap<Env, u16>> =
                HashMap::new();
            for old_entry in old.service_proxy_port_map.iter() {
                let tracked_name = ecs_arn_to_name(old_entry.0);
                let env = Env::from_any(old_entry.0);
                if !service_proxy_port_map.contains_key(&tracked_name) {
                    service_proxy_port_map.insert(tracked_name.clone(), HashMap::new());
                }
                service_proxy_port_map
                    .get_mut(&tracked_name)
                    .unwrap()
                    .insert(env, *old_entry.1);
            }

            let new_config = UserConfig {
                id: old.id,
                verson: 1,
                last_used_profile: old.last_used_profile,
                known_profiles: old.known_profiles,
                tracked_names,
                db_proxy_port_map,
                service_proxy_port_map,
                dbeaver_path: UserConfig::recheck_dbeaver_path(old.dbeaver_path),
                preffered_environments: vec![Env::DEV, Env::DEMO, Env::PROD],
            };
            new_config.save();
            let _ = fs::remove_file(UserConfigOld::config_path());
            return new_config;
        }

        let config_file = UserConfig::config_path();
        let mut user_config = match std::fs::read_to_string(config_file) {
            Ok(json) => serde_json::from_str::<UserConfig>(&json).unwrap(),
            Err(_) => UserConfig {
                id: Uuid::new_v4(),
                verson: 1,
                last_used_profile: None,
                known_profiles: HashSet::new(),
                tracked_names: HashSet::new(),
                db_proxy_port_map: HashMap::new(),
                service_proxy_port_map: HashMap::new(),
                dbeaver_path: None,
                preffered_environments: vec![Env::DEV, Env::DEMO, Env::PROD],
            },
        };
        user_config.dbeaver_path =
            UserConfig::recheck_dbeaver_path(user_config.dbeaver_path.clone());
        user_config
    }

    fn config_path() -> PathBuf {
        home::home_dir().unwrap().as_path().join(".wombat_v1")
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

    pub fn save_preffered_envs(&mut self, envs: Vec<Env>) -> Result<UserConfig, BError> {
        self.preffered_environments = envs;
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

        let used_ports: Vec<u16> = map
            .values()
            .map(|e| e.values())
            .flatten()
            .copied()
            .collect();

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
        return port.0;
    }

    pub fn get_service_port(&mut self, ecs_arn: &str) -> u16 {
        let env = Env::from_any(ecs_arn);
        let tracked_name = ecs_arn_to_name(ecs_arn);
        let port = Self::get_port(&mut self.service_proxy_port_map, tracked_name, env);
        if port.1 {
            self.save()
        }
        return port.0;
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

    pub fn use_profile(&mut self, profile: &str) {
        self.last_used_profile = Some(profile.to_owned());
        self.known_profiles.insert(profile.to_owned());
        self.save()
    }

    pub fn favorite(&mut self, tracked_name: TrackedName) -> Result<UserConfig, BError> {
        info!("Favorite {} ", &tracked_name);
        if !self.tracked_names.remove(&tracked_name) {
            info!("Favorite Add {} ", &tracked_name);
            self.tracked_names.insert(tracked_name);
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
