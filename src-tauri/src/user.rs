use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::shared::BError;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    id: Uuid,
    last_used_profile: Option<String>,
    known_profiles: HashSet<String>,
    favourite_names: HashSet<String>,
    db_proxy_port_map: HashMap<String, u16>,
    pub dbeaver_path: Option<String>,
}

impl UserConfig {
    pub fn default() -> UserConfig {
        let config_file = UserConfig::config_path();
        let user_config = match std::fs::read_to_string(config_file) {
            Ok(json) => serde_json::from_str::<UserConfig>(&json).unwrap(),
            Err(_) => UserConfig {
                id: Uuid::new_v4(),
                last_used_profile: None,
                known_profiles: HashSet::new(),
                favourite_names: HashSet::new(),
                db_proxy_port_map: HashMap::new(),
                dbeaver_path: None,
            },
        };
        user_config
    }

    fn config_path() -> PathBuf {
        home::home_dir().unwrap().as_path().join(".wombat.json")
    }

    pub fn get_db_port(&mut self, db_arn: &str) -> u16 {
        if let Some(port) = self.db_proxy_port_map.get(db_arn) {
            *port
        } else {
            let mut possible_port = rand::thread_rng().gen_range(52000..53000);
            while self.db_proxy_port_map.values().any(|p| *p == possible_port) {
                possible_port = rand::thread_rng().gen_range(52000..53000);
            }
            self.db_proxy_port_map
                .insert(db_arn.to_owned(), possible_port);
            self.save();
            possible_port
        }
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

    pub fn toggle_favourite(&mut self, name: &str) -> Result<UserConfig, BError> {
        if !self.favourite_names.remove(&name.to_owned()) {
            self.favourite_names.insert(name.to_owned());
        }

        self.save();
        Ok(self.clone())
    }

    fn save(&self) {
        std::fs::write(
            UserConfig::config_path(),
            serde_json::to_string_pretty(self).expect("Failed to serialize user config"),
        )
        .expect("Failed to save user config");
    }
}
