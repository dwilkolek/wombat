use crate::shared::{self, BError, Env};
use aws_config::{
    profile::{ProfileFileLoadError, ProfileSet},
    BehaviorVersion,
};
use aws_sdk_cloudwatchlogs as cloudwatchlogs;
use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use aws_sdk_secretsmanager as secretsmanager;
use aws_sdk_ssm as ssm;
use aws_sdk_sts as sts;
use chrono::prelude::*;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::Arc,
};
use tokio::{process::Command, sync::RwLock};
use tracing_unwrap::{OptionExt, ResultExt};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bastion {
    pub instance_id: String,
    pub env: Env,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Cluster {
    pub name: String,
    pub arn: String,
    pub env: Env,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdsInstance {
    pub arn: String,
    pub name: String,
    pub normalized_name: String,
    pub engine: String,
    pub engine_version: String,
    pub endpoint: Endpoint,
    pub environment_tag: String,
    pub env: Env,
    pub appname_tag: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct DbSecretDTO {
    dbInstanceIdentifier: String,
    pub dbname: String,
    engine: String,
    host: String,
    pub password: String,
    port: u16,
    pub username: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DbSecret {
    pub dbname: String,
    pub password: String,
    pub username: String,
    pub auto_rotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsService {
    pub arn: String,
    pub name: String,
    pub cluster_arn: String,
    pub env: Env,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetails {
    pub timestamp: DateTime<Utc>,
    pub arn: String,
    pub name: shared::TrackedName,
    pub version: String,
    pub cluster_arn: String,
    pub env: Env,
    pub task_registered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDetailsMissing {
    pub timestamp: DateTime<Utc>,
    pub arn: String,
    pub name: shared::TrackedName,
    pub error: String,
    pub env: Env,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub log_stream_name: String,
    pub timestamp: i64,
    pub ingestion_time: i64,
    pub message: String,
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
            self.region_from_profile(profile).await,
        )
        .or_default_provider()
        .or_else(aws_config::Region::new("eu-west-1"))
    }

    async fn region_from_profile(&self, profile: &str) -> Option<aws_config::Region> {
        if let Some(profile) = self.profile_set.get_profile(profile) {
            if let Some(region) = profile.get("region") {
                return Some(aws_config::Region::new(region.to_owned()));
            }
        }
        None
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

async fn profile_set() -> Result<ProfileSet, ProfileFileLoadError> {
    aws_config::profile::load(
        &aws_types::os_shim_internal::Fs::real(),
        &aws_types::os_shim_internal::Env::real(),
        &aws_runtime::env_config::file::EnvConfigFiles::default(),
        None,
    )
    .await
}

pub async fn is_logged(profile: &str, config: &aws_config::SdkConfig) -> bool {
    let sts = sts::Client::new(config);
    let sdk_result = sts.get_caller_identity().send().await;
    let sdk_ok = sdk_result.is_ok();
    let cli_ok = is_cli_logged(profile).await;
    info!("Checked is_logged profile={profile}, sdk={sdk_ok}, cli={cli_ok}");
    sdk_ok && cli_ok
}

pub async fn bastions(config: &aws_config::SdkConfig) -> Vec<Bastion> {
    let ec2_client = ec2::Client::new(config);
    let filter = ec2::types::Filter::builder()
        .name("tag:Name")
        .values("*-bastion*")
        .build();
    let res = ec2_client.describe_instances().filters(filter).send().await;

    let res = res.expect_or_log("Failed to get ec2 bastion instances");
    let res = res.reservations();
    res.iter()
        .flat_map(|r| {
            let instances = r.instances();
            instances
                .iter()
                .map(|instance| {
                    let env = Env::from_exact(
                        instance
                            .tags()
                            .iter()
                            .find(|env_tag_maybe| {
                                env_tag_maybe.key().unwrap_or("unknown").eq("Environment")
                            })
                            .map(|env_tag| env_tag.value().unwrap_or_default())
                            .unwrap_or_default(),
                    );

                    Bastion {
                        instance_id: instance.instance_id().unwrap_or_log().to_owned(),
                        env,
                    }
                })
                .collect::<Vec<Bastion>>()
        })
        .collect::<Vec<Bastion>>()
}

pub async fn db_secret(
    config: &aws_config::SdkConfig,
    name: &str,
    env: &Env,
) -> Result<DbSecret, BError> {
    let mut possible_secrets = Vec::new();
    if name.contains("-migrated") {
        possible_secrets.push(format!(
            "{}-{}/db-credentials",
            name.replace("-migrated", ""),
            env
        ));
        possible_secrets.push(format!(
            "{}-{}/spring-datasource-password",
            name.replace("-migrated", ""),
            env
        ));
        possible_secrets.push(format!(
            "{}-{}/datasource-password",
            name.replace("-migrated", ""),
            env
        ));
    }
    possible_secrets.push(format!("{}-{}/db-credentials", name, env));
    possible_secrets.push(format!("{}-{}/spring-datasource-password", name, env));
    possible_secrets.push(format!("{}-{}/datasource-password", name, env));

    let secret_client = secretsmanager::Client::new(config);

    for secret in possible_secrets {
        info!("Looking for db credentials by name: {secret}");
        let filter = secretsmanager::types::Filter::builder()
            .key("name".into())
            .values(&secret)
            .build();
        let secret_arn = secret_client.list_secrets().filters(filter).send().await;
        if secret_arn.is_err() {
            let err = secret_arn.unwrap_err();
            let error_str = err
                .source()
                .map(|s| s.to_string())
                .unwrap_or(err.to_string());
            warn!("failed to fetch secret, reason: {}", error_str);
            return Err(BError::new("db_secret", "Auth error?"));
        }
        let secret_arn = secret_arn.expect("Failed to fetch!");

        let secret_arn = secret_arn.secret_list();
        if secret_arn.len() == 1 {
            let secret_arn = secret_arn.first().unwrap_or_log();
            let secret_arn = secret_arn.arn().expect("Expected arn password");

            let secret = secret_client
                .get_secret_value()
                .secret_id(secret_arn)
                .send()
                .await;
            if secret.is_err() {
                let err = secret.unwrap_err();
                let error_str = err
                    .source()
                    .map(|s| s.to_string())
                    .unwrap_or(err.to_string());
                warn!("failed to get secret value, reason: {}", error_str);
                return Err(BError::new("db_secret", "Access denied"));
            }
            let secret = secret.unwrap_or_log();
            let secret = secret.secret_string().expect("There should be a secret");
            let secret =
                serde_json::from_str::<DbSecretDTO>(secret).expect("Deserialzied DbSecret");

            return Ok(DbSecret {
                dbname: secret.dbname,
                password: secret.password,
                username: secret.username,
                auto_rotated: true,
            });
        }
    }

    let mut possible_secrets = Vec::new();
    if name.contains("-migrated") {
        possible_secrets.push(format!(
            "/config/{}_{}/db-credentials",
            name.replace("-migrated", ""),
            env
        ));
        possible_secrets.push(format!(
            "/config/{}_{}/spring-datasource-password",
            name.replace("-migrated", ""),
            env
        ));
        possible_secrets.push(format!(
            "/config/{}_{}/datasource-password",
            name.replace("-migrated", ""),
            env
        ));
    }
    possible_secrets.push(format!("/config/{}_{}/db-credentials", name, env));
    possible_secrets.push(format!(
        "/config/{}_{}/spring-datasource-password",
        name, env
    ));
    possible_secrets.push(format!("/config/{}_{}/datasource-password", name, env));
    let ssm_client = ssm::Client::new(config);
    for secret in possible_secrets {
        let param = ssm_client
            .get_parameter()
            .name(&secret)
            .with_decryption(true)
            .send()
            .await;

        if let Ok(param) = param {
            if let Some(param) = param.parameter() {
                return Ok(DbSecret {
                    dbname: name.to_owned(),
                    password: param.value().unwrap_or_log().to_owned(),
                    username: name.to_owned(),
                    auto_rotated: false,
                });
            }
        }
    }

    Err(BError::new("db_secret", "No secret found"))
}

pub async fn get_secret(
    config: &aws_config::SdkConfig,
    secret_name: &str,
) -> Result<String, BError> {
    let ssm_client = ssm::Client::new(config);
    let param = ssm_client
        .get_parameter()
        .name(secret_name.to_owned())
        .with_decryption(true)
        .send()
        .await;

    info!("looking for secret={secret_name}");
    if let Ok(param) = param {
        if let Some(param) = param.parameter() {
            let secret = param.value().unwrap_or_log().to_owned();
            info!("Found secret with value={secret}");
            return Ok(secret);
        }
    }

    Err(BError::new("secret", "Secret not found"))
}

pub async fn databases(config: &aws_config::SdkConfig) -> Vec<RdsInstance> {
    let mut there_is_more = true;
    let mut marker = None;
    let name_regex = Regex::new(".*(play|lab|dev|demo|prod)-(.*)").unwrap_or_log();
    let rds_client = rds::Client::new(config);
    let mut databases = vec![];
    while there_is_more {
        let resp = rds_client
            .describe_db_instances()
            .set_marker(marker)
            .max_records(100)
            .send()
            .await
            .unwrap_or_log();
        marker = resp.marker().map(|m| m.to_owned());
        let rdses = resp.db_instances();
        there_is_more = rdses.len() == 100 && marker.is_some();
        rdses.iter().for_each(|rds| {
            if rds.db_name().is_some() {
                let db_instance_arn = rds.db_instance_arn().unwrap_or_log().to_owned();
                let name = name_regex
                    .captures(&db_instance_arn)
                    .and_then(|c| c.get(2))
                    .map(|c| c.as_str().to_owned())
                    .unwrap_or(db_instance_arn.split(':').last().unwrap_or_log().to_owned());
                let tags = rds.tag_list();
                let mut appname_tag = String::from("");
                let mut environment_tag = String::from("");
                let endpoint = rds
                    .endpoint()
                    .map(|e| Endpoint {
                        address: e.address().unwrap_or_log().to_owned(),
                        port: u16::try_from(e.port().unwrap_or_log()).unwrap_or_log(),
                    })
                    .unwrap_or_log()
                    .clone();
                let engine: String = rds.engine().unwrap_or("??").to_owned();
                let engine_version = format!("v{}", rds.engine_version().unwrap_or("??"));
                let mut env = Env::DEVNULL;
                for t in tags {
                    if t.key().unwrap_or_log() == "AppName" {
                        t.value().unwrap_or_log().clone_into(&mut appname_tag)
                    }
                    if t.key().unwrap_or_log() == "Environment" {
                        t.value()
                            .unwrap_or_log()
                            .to_owned()
                            .clone_into(&mut environment_tag);
                        env = Env::from_exact(&environment_tag);
                    }
                }

                let db = RdsInstance {
                    arn: db_instance_arn,
                    normalized_name: name.replace("-migrated", ""),
                    name,
                    engine,
                    engine_version,
                    endpoint,
                    appname_tag,
                    environment_tag,
                    env: env.clone(),
                };
                databases.push(db)
            }
        });
    }
    databases.sort_by(|a, b| a.name.cmp(&b.name));
    databases
}

pub async fn clusters(config: &aws_config::SdkConfig) -> Vec<Cluster> {
    let ecs_client = ecs::Client::new(config);

    info!("Fetching clusters!");
    let cluster_resp = &ecs_client
        .list_clusters()
        .send()
        .await
        .expect("Failed to get Cluster list");

    let cluster_arns = cluster_resp.cluster_arns();

    let mut clusters = vec![];
    for cluster_arn in cluster_arns {
        let env = Env::from_any(cluster_arn);
        clusters.push(Cluster {
            name: shared::cluster_arn_to_name(cluster_arn),
            arn: cluster_arn.clone(),
            env,
        });
    }

    clusters
}

pub async fn get_deploment_status(
    config: &aws_config::SdkConfig,
    cluster_arn: &str,
    service_name: &str,
    deployment_id: &str,
) -> Result<aws_sdk_ecs::types::DeploymentRolloutState, BError> {
    let ecs_client = ecs::Client::new(config);
    let result = ecs_client
        .describe_services()
        .cluster(cluster_arn)
        .services(service_name)
        .send()
        .await;

    let rollout_state = result.map(|result| {
        result.services.and_then(|services| {
            services
                .into_iter()
                .find(|service| service.service_name().unwrap_or("") == service_name)
                .and_then(|service| {
                    service
                        .deployments()
                        .iter()
                        .find(|deployment| deployment.id().unwrap_or("-") == deployment_id)
                        .and_then(|deployment| deployment.rollout_state().cloned())
                })
        })
    });

    match rollout_state {
        Err(err) => Err(shared::BError::new("deployment_status", err.to_string())),
        Ok(rollout_state) => {
            rollout_state.ok_or(BError::new("deployment_status", "missing deployment"))
        }
    }
}

pub async fn restart_service(
    config: &aws_config::SdkConfig,
    cluster_arn: &str,
    service_name: &str,
) -> Result<String, BError> {
    let ecs_client = ecs::Client::new(config);
    let result = ecs_client
        .update_service()
        .cluster(cluster_arn)
        .service(service_name)
        .force_new_deployment(true)
        .send()
        .await;
    match result {
        Ok(output) => {
            let service = output.service.unwrap_or_log();
            let deployments = &service.deployments;
            let deployment_id = deployments
                .as_ref()
                .and_then(|dpls| dpls.first())
                .and_then(|deployment| deployment.id());
            match deployment_id {
                Some(deployment_id) => Ok(deployment_id.to_owned()),
                None => Err(shared::BError::new(
                    "restart_service",
                    "missing deployment id",
                )),
            }
        }
        Err(err) => {
            let error_msg = format!(
                "failed to restart service {}, cluster: {}. Reason: {}",
                service_name, cluster_arn, err
            );
            error!("Error: {error_msg}");
            Err(shared::BError::new("restart_service", error_msg))
        }
    }
}

pub async fn services(config: &aws_config::SdkConfig, cluster: &Cluster) -> Vec<EcsService> {
    let ecs_client = ecs::Client::new(config);
    info!("Fetching services for {}", cluster.arn);
    let mut values = vec![];
    let mut has_more = true;
    let mut next_token = None;

    while has_more {
        let services_resp = ecs_client
            .list_services()
            .cluster(cluster.arn.to_owned())
            .max_results(100)
            .set_next_token(next_token)
            .send()
            .await
            .unwrap_or_log();
        next_token = services_resp.next_token().map(|t| t.to_owned());
        has_more = next_token.is_some();

        services_resp.service_arns().iter().for_each(|service_arn| {
            values.push(EcsService {
                name: service_arn.split('/').last().unwrap_or_log().to_owned(),
                arn: service_arn.to_owned(),
                cluster_arn: cluster.arn.to_owned(),
                env: cluster.env.clone(),
            })
        })
    }
    values.sort_by(|a, b| a.name.cmp(&b.name));

    values
}

pub async fn service_details(
    aws_config_provider: Arc<RwLock<AwsConfigProvider>>,
    services: Vec<EcsService>,
) -> Vec<Result<ServiceDetails, ServiceDetailsMissing>> {
    let mut result = Vec::new();
    let mut tokio_tasks = Vec::new();
    for service in services {
        let aws_config_provider = aws_config_provider.read().await;
        let (profile, config) = aws_config_provider
            .app_config_with_fallback(&service.name, &service.env)
            .await
            .expect_or_log("Config doesn't exist");
        info!("Using {profile} for {} at {}", service.arn, service.env);
        tokio_tasks.push(tokio::spawn(async move {
            service_detail(&config, service.arn).await
        }))
    }
    for handle in tokio_tasks {
        let sd = handle.await.unwrap_or_log();
        result.push(sd);
    }

    result
}

pub async fn service_detail(
    config: &aws_config::SdkConfig,
    service_arn: String,
) -> Result<ServiceDetails, ServiceDetailsMissing> {
    info!("Fetching service details for {}", service_arn);

    let ecs_client = ecs::Client::new(config);
    let cluster = service_arn.split('/').collect::<Vec<&str>>()[1];
    let service = ecs_client
        .describe_services()
        .services(&service_arn)
        .cluster(cluster)
        .send()
        .await;
    if service.is_err() {
        let err = service.unwrap_err();
        let error_str = err
            .source()
            .map(|s| s.to_string())
            .unwrap_or(err.to_string());
        error!(
            "Failed to describe service for {}, reason {}",
            &service_arn, error_str
        );
        return Err(ServiceDetailsMissing {
            name: shared::arn_to_name(&service_arn),
            timestamp: Utc::now(),
            env: Env::from_any(&service_arn),
            error: "Failed to describe service".to_owned(),
            arn: service_arn,
        });
    }
    let service = service.unwrap();
    let service = service.services();
    let service = &service[0];
    let task_def_arn = service.task_definition().unwrap_or_log();
    let task_def = ecs_client
        .describe_task_definition()
        .task_definition(task_def_arn)
        .send()
        .await;

    if task_def.is_err() {
        let err = task_def.unwrap_err();
        let error_str = err
            .source()
            .map(|s| s.to_string())
            .unwrap_or(err.to_string());
        error!(
            "Failed to fetch task definition for {}, reason {}",
            &service_arn, error_str
        );
        return Err(ServiceDetailsMissing {
            name: shared::arn_to_name(&service_arn),
            timestamp: Utc::now(),
            env: Env::from_any(&service_arn),
            error: "Failed to fetch task definition".to_owned(),
            arn: service_arn,
        });
    }
    let task_def = task_def.unwrap();

    let task_def = task_def.task_definition().unwrap_or_log();
    let container_def = &task_def.container_definitions()[0];

    let version = container_def
        .image()
        .unwrap_or_log()
        .split(':')
        .last()
        .unwrap_or("missing")
        .to_owned();

    Ok(ServiceDetails {
        name: shared::arn_to_name(&service_arn),
        timestamp: Utc::now(),
        arn: service_arn.to_owned(),
        cluster_arn: service.cluster_arn().unwrap_or_log().to_owned(),
        task_registered_at: task_def
            .registered_at
            .and_then(|t| t.to_millis().ok())
            .and_then(DateTime::from_timestamp_millis),
        version,
        env: Env::from_any(&service_arn),
    })
}

pub trait LogSearchMonitor: Send {
    fn notify(&mut self, logs: Vec<LogEntry>);
    fn success(&mut self, msg: String);
    fn error(&mut self, msg: String);
    fn message(&mut self, msg: String);
}

#[allow(clippy::too_many_arguments)]
pub async fn find_logs(
    config: &aws_config::SdkConfig,
    env: Env,
    apps: Vec<String>,
    start_date: i64,
    end_date: i64,
    filter: Option<String>,
    log_search_monitor: Arc<tokio::sync::Mutex<dyn LogSearchMonitor>>,
    limit: Option<usize>,
) -> Result<usize, BError> {
    let client = cloudwatchlogs::Client::new(config);
    let response = client
        .describe_log_groups()
        .set_log_group_name_prefix(Some(format!("dsi-{}-", env).to_owned()))
        .send()
        .await;

    let response_data = response.unwrap();
    let apps_dbg_str = format!("web/{}/", &apps.join("|"));

    let groups = response_data.log_groups();
    let mut log_count: usize = 0;

    let search_string = filter.clone().unwrap_or(String::from("<empty>"));
    let mut stream_names = Vec::new();
    info!("limit: {:?}", &limit);
    {
        let mut notifier = log_search_monitor.lock().await;
        notifier.message(String::from("Search log streams in progress..."));
    }
    for group in groups {
        let group_name = group.log_group_name().unwrap_or_default();
        info!("log group: {}", &group_name);
        {
            info!("Searching for {search_string} in {apps_dbg_str}");
            let log_streams_result = find_stream_names(
                &client,
                group_name,
                &apps,
                start_date,
                end_date,
                log_search_monitor.clone(),
            )
            .await;
            (match log_streams_result {
                Ok(names) => stream_names = names,
                Err(error) => {
                    error!("search for log streams failed, cause={error}");
                    let mut notifier = log_search_monitor.lock().await;
                    notifier.error(error);
                    return Result::Ok(0);
                }
            });

            {
                let mut notifier = log_search_monitor.lock().await;
                if stream_names.is_empty() {
                    info!("log streams empty, returning");
                    notifier.success(String::from(
                        "No log streams found having logs in given timeframe.",
                    ));
                    return Result::Ok(0);
                } else {
                    notifier.message(format!(
                        "Searching in {} log stream(s)...",
                        stream_names.len()
                    ));
                }
            }

            info!(
                "found log streams: [{}]",
                &stream_names.join(",").to_string()
            );

            let mut marker = None;
            let mut first = true;
            for chunk in stream_names.chunks(100) {
                while marker.as_ref().is_some() || first {
                    first = false;
                    let logs_response = client
                        .filter_log_events()
                        .set_log_group_name(Some(group_name.to_owned()))
                        .set_log_stream_names(Some(chunk.to_vec()))
                        .set_next_token(marker)
                        .set_filter_pattern(filter.clone())
                        .set_start_time(Some(start_date))
                        .set_end_time(Some(end_date))
                        .send()
                        .await;

                    if logs_response.is_err() {
                        let message = logs_response
                            .unwrap_err()
                            .into_service_error()
                            .meta()
                            .message()
                            .unwrap_or("")
                            .to_owned();

                        let mut notifier = log_search_monitor.lock().await;
                        notifier.error(format!("Error: {}", &message).to_owned());

                        return Result::Err(BError {
                            message,
                            command: "find_logs".to_owned(),
                        });
                    }
                    let log_response_data = logs_response.unwrap();

                    marker = log_response_data.next_token().map(|m| m.to_owned());
                    let events = log_response_data.events.unwrap_or_default();
                    info!("found {} logs", &events.len());
                    log_count += events.len();
                    let mut notifier = log_search_monitor.lock().await;
                    notifier.notify(
                        events
                            .into_iter()
                            .map(|event| LogEntry {
                                log_stream_name: event.log_stream_name.unwrap_or_default(),
                                timestamp: event.timestamp.unwrap_or_default(),
                                ingestion_time: event.ingestion_time.unwrap_or_default(),
                                message: event.message.unwrap_or_default().to_owned(),
                            })
                            .collect(),
                    );
                    notifier.message(format!(
                        "Searching in {} log stream(s), found {log_count} logs...",
                        stream_names.len()
                    ));
                    if let Some(limit) = limit {
                        if log_count > limit {
                            let msg = format!(
                                "Search in {} log stream(s) aborted, found {} logs. Reached limit of {} logs.",
                                stream_names.len(), &log_count, &limit
                            )
                            .to_owned();
                            warn!("exceeded max log count, Limit {log_count}/{limit}");
                            notifier.error(msg.to_owned());
                            return Result::Err(BError {
                                message: msg,
                                command: "find_logs".to_owned(),
                            });
                        }
                    }
                }
            }
        }
    }
    info!("logs search finished");

    let mut notifier = log_search_monitor.lock().await;
    notifier.success(format!(
        "Search in {} log stream(s) done. Found {log_count} logs.",
        stream_names.len()
    ));
    Result::Ok(log_count)
}

async fn find_stream_names(
    client: &cloudwatchlogs::Client,
    group_name: &str,
    apps: &[String],
    start_date: i64,
    end_date: i64,
    log_search_monitor: Arc<tokio::sync::Mutex<dyn LogSearchMonitor>>,
) -> Result<Vec<String>, String> {
    let mut stream_names = vec![];
    let mut streams_marker = None;
    let mut done = HashSet::new();

    loop {
        let describe_log_streams_response = client
            .describe_log_streams()
            .set_log_group_name(Some(group_name.to_owned()))
            .set_order_by(Some(cloudwatchlogs::types::OrderBy::LastEventTime))
            .set_descending(Some(true))
            .set_limit(Some(50))
            .set_next_token(streams_marker)
            .send()
            .await;

        if describe_log_streams_response.is_err() {
            let message = describe_log_streams_response.unwrap_err().to_string();
            return Err(format!("Error: {}", &message));
        }
        let data = describe_log_streams_response.unwrap();

        streams_marker = data.next_token().map(|m| m.to_owned());

        let streams = data.log_streams.unwrap_or_default();
        let mut last_creation_dates = HashMap::new();

        for stream in streams {
            let stream_name = stream.log_stream_name.unwrap_or_default();

            let app = apps
                .iter()
                .find(|app| stream_name.starts_with(&format!("web/{}/", &app)));
            if let Some(app) = app {
                let last_known_creation_time: i64 = last_creation_dates
                    .get(app)
                    .copied()
                    .unwrap_or(i64::max_value());
                let log_stream_start = stream
                    .first_event_timestamp
                    .or(stream.creation_time)
                    .unwrap_or(i64::max_value());
                let log_stream_end = stream
                    .last_event_timestamp
                    .or(stream.last_ingestion_time)
                    .map_or(i64::max_value(), |t| t + 60 * 1000);

                let overlaps = range_overlap::has_incl_overlap(
                    log_stream_start,
                    log_stream_end,
                    start_date,
                    end_date,
                );

                if last_known_creation_time > log_stream_end {
                    last_creation_dates.insert(app.clone(), log_stream_end);
                };

                info!(
                    "matched: app={app}, overlaps={overlaps}, [{} -> {}] overlaps with criteria [{} -> {}]",
                    i64_to_str(log_stream_start),
                    i64_to_str(log_stream_end),
                    i64_to_str(start_date),
                    i64_to_str(end_date),
                );
                if overlaps {
                    info!("stream {stream_name} matches name & timestamp criteria");
                    stream_names.push(stream_name.to_owned());
                    let mut log_search_monitor = log_search_monitor.lock().await;
                    log_search_monitor.message(format!(
                        "Searching for log streams, found {} stream(s). Last: {stream_name}",
                        stream_names.len()
                    ));
                }

                if log_stream_end < start_date {
                    done.insert(app.clone());
                }
            }
        }

        if apps.len() == done.len() || streams_marker.is_none() {
            break;
        }
    }

    Ok(stream_names)
}

fn i64_to_str(timestamp: i64) -> String {
    DateTime::from_timestamp_millis(timestamp)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub async fn is_cli_logged(profile: &str) -> bool {
    Command::new("aws")
        .args(["sts", "get-caller-identity", "--profile", profile])
        .output()
        .await
        .is_ok()
}

pub fn cli_login(profile: &str) {
    let mut child = std::process::Command::new("aws")
        .args(["sso", "login", "--profile", profile])
        .spawn()
        .expect("failed to execute process");

    let one_sec = core::time::Duration::from_secs(30);
    let _ = match child.wait_timeout(one_sec).unwrap() {
        Some(status) => status.code(),
        None => {
            child.kill().unwrap();
            child.wait().unwrap().code()
        }
    };
}
