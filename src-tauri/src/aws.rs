use std::{collections::HashMap, sync::Arc};

use crate::shared::{self, BError, Env};
use aws_sdk_cloudwatchlogs as cloudwatchlogs;
use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use aws_sdk_secretsmanager as secretsmanager;
use aws_sdk_ssm as ssm;
use chrono::prelude::*;
use ec2::types::Filter;
use log::{info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
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
pub struct DbInstance {
    pub name: String,
    pub engine: String,
    pub engine_version: String,
    pub endpoint: Endpoint,
    pub arn: String,
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
    pub name: String,
    pub arn: String,
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
pub struct LogEntry {
    pub log_stream_name: String,
    pub timestamp: i64,
    pub ingestion_time: i64,
    pub message: String,
}

pub async fn is_logged(config: &aws_config::SdkConfig) -> bool {
    let ecs = ecs::Client::new(&config);
    let resp = ecs.list_clusters().send().await;
    return resp.is_ok();
}

pub async fn bastions(config: &aws_config::SdkConfig) -> Vec<Bastion> {
    let ec2_client = ec2::Client::new(&config);
    let filter = Filter::builder()
        .name("tag:Name")
        .values("*-bastion*")
        .build();
    let res = ec2_client.describe_instances().filters(filter).send().await;

    let res = res.expect_or_log("Failed to get ec2 bastion instances");
    let res = res.reservations().unwrap_or_default();
    let res = res
        .iter()
        .map(|r| {
            if let Some(instances) = r.instances() {
                instances
                    .into_iter()
                    .map(|instance| {
                        let env = Env::from_exact(
                            instance
                                .tags()
                                .unwrap_or_default()
                                .into_iter()
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
            } else {
                vec![]
            }
        })
        .flatten()
        .collect::<Vec<Bastion>>();
    res
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
    dbg!(&possible_secrets);

    let secret_client = secretsmanager::Client::new(&config);

    for secret in possible_secrets {
        let filter = secretsmanager::types::Filter::builder()
            .key("name".into())
            .values(&secret)
            .build();
        let secret_arn = secret_client.list_secrets().filters(filter).send().await;

        let secret_arn = secret_arn.expect("Failed to fetch!");
        let secret_arn = secret_arn.secret_list().expect("No arn list!");
        if secret_arn.len() == 1 {
            let secret_arn = secret_arn.first().unwrap_or_log();
            let secret_arn = secret_arn.arn().expect("Expected arn password");

            let secret = secret_client
                .get_secret_value()
                .secret_id(secret_arn)
                .send()
                .await;
            if secret.is_err() {
                return Err(BError::new("db_secret", "No secrets found"));
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
    let ssm_client = ssm::Client::new(&config);
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

    return Err(BError::new("db_secret", "No secrets found"));
}

pub async fn databases(
    config: &aws_config::SdkConfig,
    database_cache: Arc<Mutex<Vec<DbInstance>>>,
) -> Vec<DbInstance> {
    {
        let database_cache = database_cache.lock().await;
        if database_cache.len() > 0 {
            return database_cache.clone();
        }
    }

    let mut there_is_more = true;
    let mut marker = None;
    let name_regex = Regex::new(".*(play|lab|dev|demo|prod)-(.*)").unwrap_or_log();
    let rds_client = rds::Client::new(&config);
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
        let instances = resp.db_instances();
        let rdses = instances.as_deref().unwrap_or_log();
        there_is_more = rdses.len() == 100 && marker.is_some();
        rdses.into_iter().for_each(|rds| {
            if let Some(_) = rds.db_name() {
                let db_instance_arn = rds.db_instance_arn().unwrap_or_log().to_owned();
                let name = name_regex
                    .captures(&db_instance_arn)
                    .and_then(|c| c.get(2))
                    .and_then(|c| Some(c.as_str().to_owned()))
                    .unwrap_or(db_instance_arn.split(":").last().unwrap_or_log().to_owned());
                let tags = rds.tag_list().unwrap_or_log();
                let mut appname_tag = String::from("");
                let mut environment_tag = String::from("");
                let endpoint = rds
                    .endpoint()
                    .map(|e| Endpoint {
                        address: e.address().unwrap_or_log().to_owned(),
                        port: u16::try_from(e.port()).unwrap_or_log(),
                    })
                    .unwrap_or_log()
                    .clone();
                let engine: String = format!("{}", rds.engine().unwrap_or("??"));
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

                let db = DbInstance {
                    name,
                    engine,
                    engine_version,
                    arn: db_instance_arn,
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
    {
        let mut database_cache = database_cache.lock().await;
        database_cache.extend(databases.clone().into_iter())
    }
    return databases;
}

pub async fn clusters(
    config: &aws_config::SdkConfig,
    cluster_cache: Arc<Mutex<Vec<Cluster>>>,
) -> Vec<Cluster> {
    {
        let cluster_cache = cluster_cache.lock().await;
        if cluster_cache.len() > 0 {
            return cluster_cache.clone();
        }
    }
    let ecs_client = ecs::Client::new(&config);

    info!("Fetching clusters!");
    let cluster_resp = &ecs_client
        .list_clusters()
        .send()
        .await
        .expect("Failed to get Cluster list");

    let cluster_arns = cluster_resp.cluster_arns().unwrap_or_default();

    let mut clusters = vec![];
    for cluster_arn in cluster_arns {
        let env = Env::from_any(cluster_arn);
        clusters.push(Cluster {
            name: shared::cluster_arn_to_name(cluster_arn),
            arn: cluster_arn.clone(),
            env: env,
        });
    }

    {
        let mut cluster_cache = cluster_cache.lock().await;
        for cluster in clusters.iter() {
            cluster_cache.push(cluster.clone());
        }
    }

    return clusters;
}

pub async fn services(
    config: &aws_config::SdkConfig,
    clusters: Vec<Cluster>,
    service_cache: Arc<Mutex<HashMap<Cluster, Vec<EcsService>>>>,
) -> HashMap<Cluster, Vec<EcsService>> {
    let mut handles = vec![];
    let mut results = HashMap::new();
    for cluster in clusters.into_iter() {
        results.insert(cluster.clone(), Vec::new());
        let cache = Arc::clone(&service_cache);
        let config = config.clone();
        let cluster = cluster.clone();
        handles.push(tokio::spawn(async move {
            (
                cluster.clone(),
                service(&config, cluster.clone(), cache).await,
            )
        }));
    }

    for handle in handles {
        let res = handle.await.unwrap_or_log();
        results.insert(res.0, res.1);
    }

    return results;
}

pub async fn service(
    config: &aws_config::SdkConfig,
    cluster: Cluster,
    service_cache: Arc<Mutex<HashMap<Cluster, Vec<EcsService>>>>,
) -> Vec<EcsService> {
    {
        if let Some(services) = service_cache.lock().await.get(&cluster) {
            return services.clone();
        }
    }

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

        services_resp
            .service_arns()
            .unwrap_or_log()
            .iter()
            .for_each(|service_arn| {
                values.push(EcsService {
                    name: service_arn.split("/").last().unwrap_or_log().to_owned(),
                    arn: service_arn.to_owned(),
                    cluster_arn: cluster.arn.to_owned(),
                    env: cluster.env.clone(),
                })
            })
    }
    values.sort_by(|a, b| a.name.cmp(&b.name));

    {
        let mut service_cache = service_cache.lock().await;
        service_cache.insert(cluster.clone(), values.clone());
    }

    values
}

pub async fn service_details(
    config: aws_config::SdkConfig,
    service_arns: Vec<String>,
    service_details_cache: Arc<Mutex<HashMap<String, ServiceDetails>>>,
) -> Vec<ServiceDetails> {
    let mut result = Vec::new();
    let mut tokio_tasks = vec![];
    for service_arn in service_arns {
        let service_details_cache = Arc::clone(&service_details_cache);
        let config = config.clone();
        tokio_tasks.push(tokio::spawn(async move {
            service_detail(config, service_arn, service_details_cache)
        }))
    }
    for handle in tokio_tasks {
        let sd = handle.await.unwrap_or_log().await;
        result.push(sd);
    }

    result
}

pub async fn service_detail(
    config: aws_config::SdkConfig,
    service_arn: String,
    service_details_cache: Arc<Mutex<HashMap<String, ServiceDetails>>>,
) -> ServiceDetails {
    {
        let cache = service_details_cache.lock().await;
        if let Some(details) = cache.get(&service_arn) {
            return details.clone();
        }
    }

    info!("Fetching service details for {}", service_arn);

    let ecs_client = ecs::Client::new(&config);
    let cluster = service_arn.split("/").collect::<Vec<&str>>()[1];
    let service = ecs_client
        .describe_services()
        .services(&service_arn)
        .cluster(cluster)
        .send()
        .await
        .unwrap_or_log();
    let service = service.services().unwrap_or_log();
    let service = &service[0];
    let task_def_arn = service.task_definition().unwrap_or_log();
    let task_def = ecs_client
        .describe_task_definition()
        .task_definition(task_def_arn)
        .send()
        .await
        .unwrap_or_log();

    let task_def = task_def.task_definition().unwrap_or_log();
    let container_def = &task_def.container_definitions().unwrap_or_log()[0];
    let version = container_def
        .image()
        .unwrap_or_log()
        .split(":")
        .last()
        .unwrap_or_log()
        .to_owned();
    let details = ServiceDetails {
        name: shared::ecs_arn_to_name(&service_arn),
        timestamp: Utc::now(),
        arn: service_arn.to_owned(),
        cluster_arn: service.cluster_arn().unwrap_or_log().to_owned(),
        version: version,
        env: Env::from_any(&service_arn),
    };
    {
        let mut cache = service_details_cache.lock().await;
        cache.insert(service_arn.to_owned(), details.clone());

        return details;
    }
}

pub trait OnLogFound: Send {
    fn notify(&self, logs: Vec<LogEntry>);
    fn success(&self);
    fn error(&self, msg: String);
}

pub async fn find_logs(
    config: &aws_config::SdkConfig,
    env: Env,
    app: String,
    start_date: i64,
    end_date: i64,
    filter: String,
    on_log_found: Arc<tokio::sync::Mutex<dyn OnLogFound>>,
) -> Result<usize, BError> {
    let client = cloudwatchlogs::Client::new(config);
    let response = client
        .describe_log_groups()
        .set_log_group_name_prefix(Some(format!("dsi-{}-", env).to_owned()))
        .send()
        .await;

    let response_data = response.unwrap();

    let groups = response_data.log_groups().unwrap_or_default();
    let mut log_count: usize = 0;
    for group in groups {
        let group_name = group.log_group_name().unwrap_or_default();
        info!("Group: {}", &group_name);
        {
            let mut stream_names = vec![];
            let mut streams_marker = None;
            let mut first_streams = true;
            info!("Looking for {} in {}", &filter, format!("web/{}/", app));
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

                        let notifier = on_log_found.lock().await;
                        notifier.error(format!("Error: {}", &message).to_owned());

                        return Result::Err(BError {
                            message: message,
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

                    if stream_names.len() > 100 || (is_over_time && found_matching_stream) {
                        streams_marker = None;
                    }
                }
            }

            info!("Found streams {:?}", &stream_names);

            let mut marker = None;
            let mut first = true;
            if stream_names.is_empty() {
                let notifier = on_log_found.lock().await;
                notifier.success();
                return Result::Ok(0);
            }

            while marker.as_ref().is_some() == true || first {
                first = false;
                let logs_response = client
                    .filter_log_events()
                    .set_log_group_name(Some(group_name.to_owned()))
                    .set_log_stream_names(Some(stream_names.clone()))
                    .set_next_token(marker)
                    .set_filter_pattern(Some(filter.clone()))
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

                    let notifier = on_log_found.lock().await;
                    notifier.error(format!("Error: {}", &message).to_owned());

                    return Result::Err(BError {
                        message: message,
                        command: "find_logs".to_owned(),
                    });
                }
                let log_response_data = logs_response.unwrap();

                marker = log_response_data.next_token().map(|m| m.to_owned());
                let events = log_response_data.events.unwrap_or_default();
                info!("LOGS Found: {}", &events.len());
                log_count = log_count + events.len();
                let notifier = on_log_found.lock().await;
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

                if log_count > 1000 {
                    warn!("LOGS Exceeded max log count");
                    notifier.error("Exceeded amount of logs. Limit 1000.".to_owned());
                    return Result::Err(BError {
                        message: "Exceeded amount of logs. Limit 1000.".to_owned(),
                        command: "find_logs".to_owned(),
                    });
                }
            }
        }
    }
    info!("LOGS Done");

    let notifier = on_log_found.lock().await;
    notifier.success();
    return Result::Ok(log_count);
}
