use std::collections::HashMap;

use crate::aws::types::Env;
use aws_config::{
    profile::{ProfileFileLoadError, ProfileSet},
    BehaviorVersion,
};
use log::{info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing_unwrap::{OptionExt, ResultExt};

pub struct AwsConfigProvider {
    pub dev_way: bool,
    profile_set: ProfileSet,
    pub wombat_profiles: Vec<WombatAwsProfile>,
    pub active_wombat_profile: WombatAwsProfile,
}

impl AwsConfigProvider {
    pub async fn new() -> Self {
        let profiles = profile_set().await.unwrap_or_log();
        let wombat_profiles = Self::load_aws_profile_configuration(&profiles);
        Self {
            dev_way: false,
            profile_set: profiles,
            active_wombat_profile: WombatAwsProfile {
                name: "default".to_owned(),
                profile_base_name: "default".to_owned(),
                sso_profiles: HashMap::new(),
                support_level: SupportLevel::None,
                single_source_profile: true,
            },
            wombat_profiles,
        }
    }

    pub fn login(&mut self, profile_name: String, dev_way: bool) {
        self.dev_way = dev_way;
        self.active_wombat_profile = self
            .wombat_profiles
            .iter()
            .find(|sso| sso.name == profile_name)
            .unwrap_or_log()
            .clone();
    }

    pub fn configured_envs(&self) -> Vec<Env> {
        self.active_wombat_profile
            .sso_profiles
            .keys()
            .cloned()
            .collect()
    }

    pub async fn sso_config(&self, env: &Env) -> (String, aws_config::SdkConfig) {
        info!("getting sso_profile for env={env}");
        let sso_profile = self
            .active_wombat_profile
            .sso_profiles
            .get(env)
            .cloned()
            .expect("Failed to get sso_profile");
        self.for_sso(&sso_profile).await
    }

    async fn get_or_create_app_config(
        &self,
        app: &str,
        env: &Env,
    ) -> Option<(String, aws_config::SdkConfig)> {
        let matching_infra_profile =
            self.active_wombat_profile
                .sso_profiles
                .get(env)
                .and_then(|sso_profile| {
                    sso_profile
                        .infra_profiles
                        .iter()
                        .find(|infra_profile| &infra_profile.env == env && infra_profile.app == app)
                        .cloned()
                });

        match matching_infra_profile {
            Some(infra_profile) => self.for_infra(&infra_profile).await,
            None => None,
        }
    }

    async fn for_sso(&self, sso_profile: &SsoProfile) -> (String, aws_config::SdkConfig) {
        self.use_aws_config(&sso_profile.profile_name).await
    }

    pub async fn for_infra(
        &self,
        infra_profile: &InfraProfile,
    ) -> Option<(String, aws_config::SdkConfig)> {
        Some(self.use_aws_config(&infra_profile.profile_name).await)
    }

    pub async fn with_dev_way_check(
        &self,
        infra_profile: &Option<InfraProfile>,
        sso_profile: &Option<SsoProfile>,
    ) -> Option<(String, aws_config::SdkConfig)> {
        info!(
            "selecting profile with dev_way check. infra={infra_profile:?}, sso={infra_profile:?}"
        );
        if let Some(infra_profile) = infra_profile {
            let infra_config = self.for_infra(infra_profile).await;
            if infra_config.is_some() {
                info!("returing infra sdk_config");
                return infra_config;
            }
        }
        if self.dev_way {
            if let Some(sso_profile) = sso_profile {
                info!("using dev_way, returing sso sdk_config");
                return Some(self.for_sso(sso_profile).await);
            }
        }

        warn!("selecting profile with dev_way check resulted in None");
        None
    }

    pub async fn app_config(
        &self,
        app: &str,
        env: &Env,
    ) -> Option<(String, aws_config::SdkConfig)> {
        let app_config = self.get_or_create_app_config(app, env).await;
        if self.dev_way {
            let user_config = Some(self.sso_config(env).await);
            app_config.or(user_config)
        } else {
            app_config
        }
    }

    pub async fn app_config_with_fallback(
        &self,
        app: &str,
        env: &Env,
    ) -> Option<(String, aws_config::SdkConfig)> {
        let app_config = self.app_config(app, env).await;
        let user_config = Some(self.sso_config(env).await);
        app_config.or(user_config)
    }

    fn load_aws_profile_configuration(profile_set: &ProfileSet) -> Vec<WombatAwsProfile> {
        let mut wombat_profiles: HashMap<String, WombatAwsProfile> = HashMap::new();
        let infra_profiles = Self::read_infra_profiles(profile_set);

        for profile in profile_set.profiles() {
            if let Some(profile_details) = profile_set.get_profile(profile) {
                let role_arn = profile_details.get("role_arn");
                let is_sso_profile = profile_details.get("sso_start_url").is_some();
                let sso_account_id = profile_details.get("sso_account_id").unwrap_or("0");
                let valid_sso_profile = match role_arn {
                    Some(role) => !role.ends_with("-infra") && is_sso_profile,
                    None => is_sso_profile,
                };
                let infra_profiles = infra_profiles
                    .iter()
                    .filter(|infra| infra.source_profile == profile)
                    .cloned()
                    .collect();
                if valid_sso_profile {
                    let sso_profile = SsoProfile {
                        profile_name: profile.to_owned(),
                        region: profile_details.get("region").map(|r| r.to_owned()),
                        infra_profiles,
                        sso_account_id: sso_account_id.to_owned(),
                        support_level: match sso_account_id {
                            "835811189142" => SupportLevel::Full,
                            "590184069535" => SupportLevel::Partial,
                            "275464048518" => SupportLevel::Partial,
                            _ => SupportLevel::None,
                        },
                        env: Env::DEVNULL,
                    };
                    let provile_env_regex =
                        Regex::new("^(.*)-(play|lab|dev|demo|prod)$").unwrap_or_log();
                    match provile_env_regex.captures(profile) {
                        Some(caps) => {
                            let base_profile_name = &caps[1];
                            let env = Env::from_exact(&caps[2]);
                            let env_sso_profile = sso_profile.for_env(env.clone());
                            match wombat_profiles.get_mut(base_profile_name) {
                                Some(existing) => existing.extend(env_sso_profile),
                                None => {
                                    let support_level = env_sso_profile.support_level.clone();
                                    let mut sso_profiles = HashMap::new();
                                    sso_profiles.insert(env.clone(), env_sso_profile);
                                    wombat_profiles.insert(
                                        base_profile_name.to_owned(),
                                        WombatAwsProfile {
                                            name: base_profile_name.to_owned(),
                                            profile_base_name: base_profile_name.to_owned(),
                                            sso_profiles,
                                            support_level,
                                            single_source_profile: true,
                                        },
                                    );
                                }
                            }
                        }
                        None => {
                            let support_level = sso_profile.support_level.clone();
                            let mut sso_profiles = HashMap::new();
                            sso_profiles.insert(Env::DEV, sso_profile.for_env(Env::DEV));
                            sso_profiles.insert(Env::DEMO, sso_profile.for_env(Env::DEMO));
                            sso_profiles.insert(Env::PROD, sso_profile.for_env(Env::PROD));
                            wombat_profiles.insert(
                                profile.to_owned(),
                                WombatAwsProfile {
                                    name: profile.to_owned(),
                                    profile_base_name: profile.to_owned(),
                                    sso_profiles,
                                    support_level,
                                    single_source_profile: true,
                                },
                            );
                        }
                    }
                }
            }
        }
        wombat_profiles
            .values()
            .filter(|w| !w.sso_profiles.is_empty())
            .cloned()
            .collect()
    }

    fn read_infra_profiles(profile_set: &ProfileSet) -> Vec<InfraProfile> {
        let mut infra_profiles: Vec<InfraProfile> = Vec::new();
        for profile in profile_set.profiles() {
            info!("analizying potential infra profile: {profile}");
            if let Some(profile_details) = profile_set.get_profile(profile) {
                let source_profile = match profile_details.get("source_profile") {
                    Some(source_profile) => {
                        info!("\t source_profile: {source_profile}");
                        source_profile
                    }
                    None => {
                        warn!("\t missing source_profile");
                        continue;
                    }
                };
                let role_arn = match profile_details.get("role_arn") {
                    Some(role_arn) => {
                        info!("\t role_arn: {role_arn}");
                        role_arn
                    }
                    None => {
                        warn!("\t missing role_arn");
                        continue;
                    }
                };

                let app_regex = Regex::new(r".*/(.*)-infra").unwrap();
                let app = match app_regex.captures(role_arn) {
                    None => {
                        warn!("\t failed to read app from role");
                        continue;
                    }
                    Some(caps) => {
                        let app = caps.get(1).map(|app| app.as_str().to_owned()).unwrap();
                        info!("\t app: {app}");
                        app
                    }
                };

                let region = profile_details.get("region");
                info!("\t region: {}", region.unwrap_or("none"));

                let mut matched_env = false;
                let base_infra_profile = InfraProfile {
                    app: app.to_owned(),
                    env: Env::DEVNULL,
                    profile_name: profile.to_owned(),
                    region: region.map(|r| r.to_owned()),
                    source_profile: source_profile.to_owned(),
                };

                if profile.ends_with("-play") {
                    info!("\t adding for play");
                    infra_profiles.push(base_infra_profile.for_env(Env::PLAY));
                    matched_env = true;
                }
                if profile.ends_with("-lab") {
                    info!("\t adding for lab");
                    infra_profiles.push(base_infra_profile.for_env(Env::LAB));
                    matched_env = true;
                }
                if profile.ends_with("-dev") {
                    info!("\t adding for dev");
                    infra_profiles.push(base_infra_profile.for_env(Env::DEV));
                    matched_env = true;
                }
                if profile.ends_with("-demo") {
                    info!("\t adding for demo");
                    infra_profiles.push(base_infra_profile.for_env(Env::DEMO));
                    matched_env = true;
                }
                if profile.ends_with("-prod") {
                    info!("\t adding for prod");
                    infra_profiles.push(base_infra_profile.for_env(Env::PROD));
                    matched_env = true;
                }
                if !matched_env {
                    info!("\t didn't match any env, adding as default for all envs");
                    infra_profiles.push(base_infra_profile.for_env(Env::PLAY));
                    infra_profiles.push(base_infra_profile.for_env(Env::LAB));
                    infra_profiles.push(base_infra_profile.for_env(Env::DEV));
                    infra_profiles.push(base_infra_profile.for_env(Env::DEMO));
                    infra_profiles.push(base_infra_profile.for_env(Env::PROD));
                }
            }
        }
        infra_profiles
    }

    pub async fn get_region(&self, profile: &str) -> String {
        let region_provider = self.region_provider(profile).await;
        region_provider.region().await.unwrap_or_log().to_string()
    }

    async fn region_provider(
        &self,
        profile: &str,
    ) -> aws_config::meta::region::RegionProviderChain {
        aws_config::meta::region::RegionProviderChain::first_try(
            region_from_profile(profile, &self.profile_set).await,
        )
        .or_default_provider()
        .or_else(aws_config::Region::new("eu-west-1"))
    }

    async fn use_aws_config(&self, ssm_profile: &str) -> (String, aws_config::SdkConfig) {
        let region_provider = self.region_provider(ssm_profile).await;

        (
            ssm_profile.to_owned(),
            aws_config::defaults(BehaviorVersion::latest())
                .profile_name(ssm_profile)
                .region(region_provider)
                .load()
                .await,
        )
    }
}

async fn region_from_profile(
    profile: &str,
    profile_set: &ProfileSet,
) -> Option<aws_config::Region> {
    if let Some(profile) = profile_set.get_profile(profile) {
        if let Some(region) = profile.get("region") {
            return Some(aws_config::Region::new(region.to_owned()));
        }
    }
    None
}

#[cfg(not(feature = "arh"))]
async fn region_provider(
    profile: &str,
    profile_set: &ProfileSet,
) -> aws_config::meta::region::RegionProviderChain {
    aws_config::meta::region::RegionProviderChain::first_try(
        region_from_profile(profile, profile_set).await,
    )
    .or_default_provider()
    .or_else(aws_config::Region::new("eu-west-1"))
}

#[cfg(not(feature = "arh"))]
pub async fn sdk_config(profile_name: &str) -> aws_config::SdkConfig {
    let profiles = profile_set().await.unwrap_or_log();
    aws_config::defaults(BehaviorVersion::latest())
        .profile_name(profile_name)
        .region(region_provider(profile_name, &profiles).await)
        .load()
        .await
}

async fn profile_set() -> Result<ProfileSet, ProfileFileLoadError> {
    aws_config::profile::load(
        &aws_types::os_shim_internal::Fs::real(),
        &aws_types::os_shim_internal::Env::real(),
        &aws_runtime::env_config::file::EnvConfigFiles::default(),
        None,
    )
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupportLevel {
    Full,
    Partial,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WombatAwsProfile {
    pub name: String,
    profile_base_name: String,
    pub sso_profiles: HashMap<Env, SsoProfile>,
    pub support_level: SupportLevel,
    pub single_source_profile: bool,
}
impl WombatAwsProfile {
    fn extend(&mut self, sso_profile: SsoProfile) {
        let new_sso_profile = sso_profile.profile_name.clone();
        self.sso_profiles
            .insert(sso_profile.env.clone(), sso_profile);
        let mut envs = self.sso_profiles.keys().cloned().collect::<Vec<_>>();
        envs.sort();

        let mut env_str = "".to_owned();
        for env in envs.iter() {
            if env_str.is_empty() {
                env_str = format!("{env}");
            } else {
                env_str = format!("{}|{}", env_str, env);
            }
        }
        self.single_source_profile = self
            .sso_profiles
            .iter()
            .all(|s| s.1.profile_name == new_sso_profile);
        let support_level_value: usize = self
            .sso_profiles
            .iter()
            .map(|s| match s.1.support_level {
                SupportLevel::Full => 1,
                SupportLevel::None => 0,
                SupportLevel::Partial => 0,
            })
            .sum();
        self.support_level = if support_level_value == self.sso_profiles.len() {
            SupportLevel::Full
        } else if support_level_value == 0 {
            SupportLevel::None
        } else {
            SupportLevel::Partial
        };
        self.name = format!("{}-({})", self.profile_base_name, env_str);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProfile {
    pub profile_name: String,
    pub region: Option<String>,
    pub infra_profiles: Vec<InfraProfile>,
    pub sso_account_id: String,
    pub support_level: SupportLevel,
    pub env: Env,
}
impl SsoProfile {
    fn for_env(&self, env: Env) -> Self {
        let mut clone = self.clone();
        clone.env = env.clone();
        clone.infra_profiles.retain(|infra| infra.env == env);
        clone
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraProfile {
    pub source_profile: String,
    pub profile_name: String,
    pub region: Option<String>,
    pub app: String,
    pub env: Env,
}

impl InfraProfile {
    fn for_env(&self, env: Env) -> Self {
        let mut clone = self.clone();
        clone.env = env;
        clone
    }
}
