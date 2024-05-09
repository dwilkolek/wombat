use std::{collections::HashMap, error::Error, str::FromStr, sync::Arc};

use crate::shared::{self, BError, Env};
use aws_config::{
    profile::{Profile, ProfileFileLoadError, ProfileSet},
    BehaviorVersion,
};
use aws_sdk_cloudwatchlogs as cloudwatchlogs;
use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use aws_sdk_secretsmanager as secretsmanager;
use aws_sdk_ssm as ssm;
use aws_types::app_name;
use chrono::prelude::*;
use ec2::types::Filter;
use log::{error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing_unwrap::{OptionExt, ResultExt};

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

pub struct AwsConfigProvider {
    pub dev_way: bool,
    profile_set: ProfileSet,
    profile: String,
    pub sso_profiles: Vec<String>,
    pub app_env_to_infra_profile: HashMap<(String, Env), String>,
    profile_to_config: HashMap<String, aws_config::SdkConfig>,
}

impl AwsConfigProvider {
    pub async fn new() -> Self {
        let profiles = profile_set().await.unwrap_or_log();
        let app_env_to_infra_profile = Self::get_app_env_to_profile_mapping(&profiles);
        let sso_profiles = Self::available_sso_profiles(&profiles);
        Self {
            dev_way: false,
            profile_set: profiles,
            profile: "default".to_owned(),
            app_env_to_infra_profile,
            profile_to_config: HashMap::new(),
            sso_profiles,
        }
    }

    pub fn login(&mut self, profile: String, dev_way: bool) {
        self.profile = profile;
        self.dev_way = dev_way;
        self.profile_to_config.clear();
    }
    pub async fn get_user_config(&mut self) -> (String, aws_config::SdkConfig) {
        let profile_config = self.profile_to_config.get(&self.profile);

        match profile_config {
            Some(config) => {
                info!("returning cached user config");
                (self.profile.clone(), config.clone())
            }
            None => {
                info!("creating user config");
                let config = self.use_aws_config(&self.profile).await;
                self.profile_to_config
                    .insert(config.0.clone(), config.1.clone());
                config
            }
        }
    }
    pub async fn get_app_config(
        &mut self,
        app: &str,
        env: &Env,
    ) -> (String, aws_config::SdkConfig) {
        match self
            .app_env_to_infra_profile
            .get(&(app.to_owned(), env.to_owned()))
        {
            Some(profile) => {
                let config = self.profile_to_config.get(profile);
                match config {
                    Some(config) => {
                        info!("returning cached {profile} config");
                        Some((profile.clone(), config.clone()))
                    }
                    None => {
                        info!("creating {profile} config");
                        let config = self.use_aws_config(&profile).await;
                        self.profile_to_config
                            .insert(config.0.clone(), config.1.clone());
                        Some(config)
                    }
                }
            }
            None => None,
        }
        .expect_or_log("Config doesn't exist")
    }
    pub async fn get_config(&mut self, app: &str, env: &Env) -> (String, aws_config::SdkConfig) {
        if self.dev_way {
            return self.get_user_config().await;
        }
        self.get_app_config(app, env).await
    }

    fn available_sso_profiles(profile_set: &ProfileSet) -> Vec<String> {
        let mut profiles: Vec<String> = profile_set.profiles().map(|p| p.to_owned()).collect();
        profiles.retain(|profile| {
            if let Some(profile_details) = profile_set.get_profile(profile.as_str()) {
                let role_arn = profile_details.get("role_arn");
                let is_sso_profile = profile_details.get("sso_start_url").is_some();
                return match role_arn {
                    Some(role) => !role.ends_with("-infra") && is_sso_profile,
                    None => is_sso_profile,
                };
            }
            false
        });

        profiles
    }

    fn get_app_env_to_profile_mapping(profile_set: &ProfileSet) -> HashMap<(String, Env), String> {
        let mut app_env_to_profile_mapping: HashMap<(String, Env), String> = HashMap::new();

        profile_set.profiles().for_each(|profile| {
            info!("analyzing profile: {profile}");
            if let Some(profile_details) = profile_set.get_profile(profile) {
                if let Some(role) = profile_details.get("role_arn") {
                    info!("\t role: {role}");
                    let app_regex = Regex::new(r".*/(.*)-infra").unwrap();
                    match app_regex.captures(role) {
                        None => {
                            warn!("failed to find app")
                        }
                        Some(caps) => {
                            let app = &caps[1];
                            info!("\t app: {app}");
                            let mut matched_env = false;
                            if profile.ends_with("-play") {
                                info!("\t adding for play");
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::PLAY), profile.to_owned());
                                matched_env = true;
                            }
                            if profile.ends_with("-lab") {
                                info!("\t adding for lab");
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::LAB), profile.to_owned());
                                matched_env = true;
                            }
                            if profile.ends_with("-dev") {
                                info!("\t adding for dev");
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::DEV), profile.to_owned());
                                matched_env = true;
                            }
                            if profile.ends_with("-demo") {
                                info!("\t adding for demo");
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::DEMO), profile.to_owned());
                                matched_env = true;
                            }
                            if profile.ends_with("-prod") {
                                info!("\t adding for prod");
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::PROD), profile.to_owned());
                                matched_env = true;
                            }
                            if !matched_env {
                                info!("\t didn't match any env, adding as default for all envs");
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::PLAY), profile.to_owned());
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::LAB), profile.to_owned());
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::DEV), profile.to_owned());
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::DEMO), profile.to_owned());
                                app_env_to_profile_mapping
                                    .insert((app.to_owned(), Env::PROD), profile.to_owned());
                            }
                        }
                    }
                }
            }
        });

        app_env_to_profile_mapping
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

// async fn region_provider(profile: &str) -> aws_config::meta::region::RegionProviderChain {
//     aws_config::meta::region::RegionProviderChain::first_try(region_from_profile(profile).await)
//         .or_default_provider()
//         .or_else(aws_config::Region::new("eu-west-1"))
// }

async fn profile_set() -> Result<ProfileSet, ProfileFileLoadError> {
    aws_config::profile::load(
        &aws_types::os_shim_internal::Fs::real(),
        &aws_types::os_shim_internal::Env::real(),
        &aws_runtime::env_config::file::EnvConfigFiles::default(),
        None,
    )
    .await
}

// async fn region_from_profile(profile: &str) -> Option<aws_config::Region> {
//     if let Ok(cf) = profile_set().await {
//         if let Some(profile) = cf.get_profile(profile) {
//             if let Some(region) = profile.get("region") {
//                 return Some(aws_config::Region::new(region.to_owned()));
//             }
//         }
//     }
//     None
// }

// pub async fn use_aws_config(ssm_profile: &str) -> (String, aws_config::SdkConfig) {
//     let region_provider = region_provider(ssm_profile).await;

//     (
//         ssm_profile.to_owned(),
//         aws_config::defaults(BehaviorVersion::latest())
//             .profile_name(ssm_profile)
//             .region(region_provider)
//             .load()
//             .await,
//     )
// }

// pub async fn available_infra_profiles() -> Vec<String> {
//     let profile_set = profile_set().await;
//     match profile_set {
//         Ok(profile_set) => {
//             let mut profiles: Vec<String> = profile_set.profiles().map(|p| p.to_owned()).collect();
//             profiles.retain(|profile| {
//                 if let Some(profile_details) = profile_set.get_profile(profile.as_str()) {
//                     if let Some(role) = profile_details.get("role_arn") {
//                         let valid_role =
//                             role.ends_with(format!("/{}-infra", profile.as_str()).as_str());
//                         if !valid_role {
//                             // warn!("Profile {} has invalid role: {}", profile, role);
//                         }
//                         return valid_role;
//                     }
//                 }
//                 false
//             });

//             profiles
//         }
//         Err(_) => vec![],
//     }
// }

// pub async fn available_sso_profiles() -> Vec<String> {
//     let profile_set = profile_set().await;
//     match profile_set {
//         Ok(profile_set) => {
//             let mut profiles: Vec<String> = profile_set.profiles().map(|p| p.to_owned()).collect();
//             profiles.retain(|profile| {
//                 if let Some(profile_details) = profile_set.get_profile(profile.as_str()) {
//                     let role_arn = profile_details.get("role_arn");
//                     let is_sso_profile = profile_details.get("sso_start_url").is_some();
//                     return match role_arn {
//                         Some(role) => !role.ends_with("-infra") && is_sso_profile,
//                         None => is_sso_profile,
//                     };
//                 }
//                 false
//             });

//             profiles
//         }
//         Err(_) => vec![],
//     }
// }

pub async fn is_logged(config: &aws_config::SdkConfig) -> bool {
    let ecs = ecs::Client::new(config);
    let resp = ecs.list_clusters().send().await;
    resp.is_ok()
}

pub async fn bastions(config: &aws_config::SdkConfig) -> Vec<Bastion> {
    let ec2_client = ec2::Client::new(config);
    let filter = Filter::builder()
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
        let filter = secretsmanager::types::Filter::builder()
            .key("name".into())
            .values(&secret)
            .build();
        let secret_arn = secret_client.list_secrets().filters(filter).send().await;
        if secret_arn.is_err() {
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
                return Err(BError::new("db_secret", "No secret found"));
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

    if let Ok(param) = param {
        if let Some(param) = param.parameter() {
            return Ok(param.value().unwrap_or_log().to_owned());
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
                        appname_tag = t.value().unwrap_or_log().to_owned()
                    }
                    if t.key().unwrap_or_log() == "Environment" {
                        environment_tag = t.value().unwrap_or_log().to_owned();
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

pub async fn services(config: &aws_config::SdkConfig, clusters: Vec<Cluster>) -> Vec<EcsService> {
    let mut handles = vec![];
    let mut results = vec![];
    for cluster in clusters.into_iter() {
        let config = config.clone();
        let cluster = cluster.clone();
        handles.push(tokio::spawn(async move {
            service(&config, cluster.clone()).await
        }));
    }

    for handle in handles {
        let res = handle.await.unwrap_or_log();
        results.extend(res);
    }

    results
}

pub async fn service(config: &aws_config::SdkConfig, cluster: Cluster) -> Vec<EcsService> {
    let ecs_client = ecs::Client::new(config);
    info!("Fetching services for {}", &cluster.arn);
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
    config: aws_config::SdkConfig,
    service_arns: Vec<String>,
) -> Vec<Result<ServiceDetails, ServiceDetailsMissing>> {
    let mut result = Vec::new();
    let mut tokio_tasks = Vec::new();
    for service_arn in service_arns {
        let config = config.clone();
        tokio_tasks.push(tokio::spawn(async move {
            service_detail(&config, service_arn).await
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
            name: shared::ecs_arn_to_name(&service_arn),
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
            name: shared::ecs_arn_to_name(&service_arn),
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
        name: shared::ecs_arn_to_name(&service_arn),
        timestamp: Utc::now(),
        arn: service_arn.to_owned(),
        cluster_arn: service.cluster_arn().unwrap_or_log().to_owned(),
        version,
        env: Env::from_any(&service_arn),
    })
}

pub trait OnLogFound: Send {
    fn notify(&mut self, logs: Vec<LogEntry>);
    fn success(&mut self);
    fn error(&mut self, msg: String);
}

#[allow(clippy::too_many_arguments)]
pub async fn find_logs(
    config: &aws_config::SdkConfig,
    env: Env,
    apps: Vec<String>,
    start_date: i64,
    end_date: i64,
    filter: Option<String>,
    on_log_found: Arc<tokio::sync::Mutex<dyn OnLogFound>>,
    limit: Option<usize>,
) -> Result<usize, BError> {
    let client = cloudwatchlogs::Client::new(config);
    let response = client
        .describe_log_groups()
        .set_log_group_name_prefix(Some(format!("dsi-{}-", env).to_owned()))
        .send()
        .await;

    let response_data = response.unwrap();
    let apps_dbg_str = format!("web/{:#?}/", &apps);

    let groups = response_data.log_groups();
    let mut log_count: usize = 0;

    info!("Limit: {:?}", &limit);
    for group in groups {
        let group_name = group.log_group_name().unwrap_or_default();
        info!("Group: {}", &group_name);
        {
            let mut stream_names = vec![];
            let mut streams_marker = None;
            let mut first_streams = true;
            info!("Looking for {:?} in {}", &filter, apps_dbg_str);
            while first_streams || streams_marker.is_some() {
                {
                    first_streams = false;
                    let response2 = client
                        .describe_log_streams()
                        .set_log_group_name(Some(group_name.to_owned()))
                        .set_next_token(streams_marker)
                        .order_by(cloudwatchlogs::types::OrderBy::LastEventTime)
                        .descending(true)
                        .send()
                        .await;

                    if response2.is_err() {
                        let message = response2.unwrap_err().to_string();

                        let mut notifier = on_log_found.lock().await;
                        notifier.error(format!("Error: {}", &message).to_owned());

                        return Result::Err(BError {
                            message,
                            command: "find_logs".to_owned(),
                        });
                    }
                    let response2_data = response2.unwrap();

                    streams_marker = response2_data.next_token().map(|m| m.to_owned());

                    let streams = response2_data.log_streams.unwrap_or_default();
                    let mut is_over_time = false;
                    let mut found_matching_stream = false;

                    for stream in streams {
                        let stream_name = stream.log_stream_name.unwrap_or_default();
                        for app in apps.iter() {
                            if stream_name.starts_with(&format!("web/{}/", &app)) {
                                let overlap = stream
                                    .first_event_timestamp
                                    .is_some_and(|ts| ts <= end_date)
                                    && stream
                                        .last_event_timestamp
                                        .is_some_and(|ts| ts >= start_date);
                                stream_names.push(stream_name.to_owned());
                                is_over_time = !overlap;
                                found_matching_stream = true
                            }
                        }
                    }

                    if stream_names.len() > 100 || (is_over_time && found_matching_stream) {
                        streams_marker = None;
                    }
                }
            }

            info!("Found streams {:?}", &stream_names);

            let mut marker = None;
            let mut first = true;
            if stream_names.is_empty() {
                let mut notifier = on_log_found.lock().await;
                notifier.success();
                return Result::Ok(0);
            }

            while marker.as_ref().is_some() || first {
                first = false;
                let logs_response = client
                    .filter_log_events()
                    .set_log_group_name(Some(group_name.to_owned()))
                    .set_log_stream_names(Some(stream_names.clone()))
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

                    let mut notifier = on_log_found.lock().await;
                    notifier.error(format!("Error: {}", &message).to_owned());

                    return Result::Err(BError {
                        message,
                        command: "find_logs".to_owned(),
                    });
                }
                let log_response_data = logs_response.unwrap();

                marker = log_response_data.next_token().map(|m| m.to_owned());
                let events = log_response_data.events.unwrap_or_default();
                info!("LOGS Found: {}", &events.len());
                log_count += events.len();
                let mut notifier = on_log_found.lock().await;
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
                if let Some(limit) = limit {
                    if log_count > limit {
                        warn!("LOGS Exceeded max log count");
                        let msg =
                            format!("Exceeded amount of logs. Limit {}/{}.", &log_count, &limit)
                                .to_owned();
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
    info!("LOGS Done");

    let mut notifier = on_log_found.lock().await;
    notifier.success();
    Result::Ok(log_count)
}
