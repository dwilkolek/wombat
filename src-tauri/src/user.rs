use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::shared::BError;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    id: Uuid,
    last_used_profile: Option<String>,
    known_profiles: HashSet<String>,
    favourite_service_names: HashSet<String>,
    favourite_db_arns: HashSet<String>,
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
                favourite_db_arns: HashSet::new(),
                favourite_service_names: HashSet::new(),
                dbeaver_path: None,
            },
        };
        user_config
    }

    fn config_path() -> PathBuf {
        home::home_dir().unwrap().as_path().join(".wombat.json")
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

    pub fn toggle_service_favourite(&mut self, service_name: &str) -> Result<UserConfig, BError> {
        if !self
            .favourite_service_names
            .remove(&service_name.to_owned())
        {
            self.favourite_service_names.insert(service_name.to_owned());
        }

        self.save();
        Ok(self.clone())
    }

    pub fn toggle_db_favourite(&mut self, db_arn: &str) -> Result<UserConfig, BError> {
        if !self.favourite_db_arns.remove(&db_arn.to_owned()) {
            self.favourite_db_arns.insert(db_arn.to_owned());
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
