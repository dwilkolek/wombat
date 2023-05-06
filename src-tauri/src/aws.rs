use std::{
    collections::{HashMap, HashSet},
    process::Command,
};

use crate::shared::{self, BError, Env};
use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use aws_sdk_secretsmanager as secretsmanager;
use aws_sdk_ssm as ssm;
use chrono::prelude::*;
use ec2::types::Filter;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bastion {
    pub arn: String,
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
    pub endpoint: Endpoint,
    pub arn: String,
    pub environment_tag: String,
    pub env: Env,
    pub appname_tag: String,
}

#[derive(Debug, Deserialize, Serialize)]
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

pub async fn check_login_and_trigger(profile: &str) -> Result<(), BError> {
    let client = ecs_client(profile).await;
    if !is_logged(&client).await {
        println!("Trigger log in into AWS");
        Command::new("aws")
            .args(["sso", "login", "--profile", &profile])
            .output()
            .expect("failed to execute process");
    }
    if !is_logged(&client).await {
        Err(BError::new("login", "Failed to log in"))
    } else {
        Ok(())
    }
}

pub async fn is_logged(ecs: &ecs::Client) -> bool {
    let resp = ecs.list_clusters().send().await;
    return resp.is_ok();
}

pub async fn ecs_client(profile: &str) -> ecs::Client {
    let config = aws_config::from_env().profile_name(profile).load().await;
    ecs::Client::new(&config)
}

pub async fn rds_client(profile: &str) -> rds::Client {
    let config = aws_config::from_env().profile_name(profile).load().await;
    rds::Client::new(&config)
}

pub async fn ec2_client(profile: &str) -> ec2::Client {
    let config = aws_config::from_env().profile_name(profile).load().await;
    ec2::Client::new(&config)
}

pub async fn secretsmanager_client(profile: &str) -> secretsmanager::Client {
    let config = aws_config::from_env().profile_name(profile).load().await;
    secretsmanager::Client::new(&config)
}

pub async fn ssm_client(profile: &str) -> ssm::Client {
    let config = aws_config::from_env().profile_name(profile).load().await;
    ssm::Client::new(&config)
}

pub async fn bastions(ec2: &ec2::Client) -> Vec<Bastion> {
    let filter = Filter::builder()
        .name("tag:Name")
        .values("*-bastion*")
        .build();
    let res = ec2.describe_instances().filters(filter).send().await;
    let res = res.expect("Failed to get bastion list");
    let res = res.reservations().unwrap();
    let res = res
        .iter()
        .map(|r| {
            if let Some(instances) = r.instances() {
                instances
                    .into_iter()
                    .map(|instance| {
                        let arn = instance
                            .iam_instance_profile()
                            .unwrap()
                            .arn()
                            .unwrap()
                            .to_owned();
                        let env = Env::from_any(&arn);
                        Bastion {
                            arn,
                            instance_id: instance.instance_id().unwrap().to_owned(),
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

pub struct RdsClient {
    rds_client: Option<rds::Client>,
    ssm_client: Option<ssm::Client>,
    secret_client: Option<secretsmanager::Client>,
    databases: Vec<DbInstance>,
}

impl RdsClient {
    pub fn new() -> Self {
        RdsClient {
            rds_client: None,
            ssm_client: None,
            secret_client: None,
            databases: Vec::new(),
        }
    }
    pub async fn init(&mut self, profile: &str) {
        self.rds_client = Some(rds_client(profile).await);
        self.ssm_client = Some(ssm_client(profile).await);
        self.secret_client = Some(secretsmanager_client(profile).await);
    }
    pub fn shutdown(&mut self) {
        self.rds_client = None;
        self.ssm_client = None;
        self.secret_client = None;
        self.clear();
    }
    pub fn clear(&mut self) {
        self.databases = Vec::new();
    }
    pub async fn databases(&mut self) -> Vec<DbInstance> {
        if self.databases.len() > 0 {
            return self.databases.clone();
        }

        let mut there_is_more = true;
        let mut marker = None;
        let name_regex = Regex::new(".*(play|lab|dev|demo|prod)-(.*)").unwrap();
        let rds_client = self.rds_client.as_ref().unwrap();
        while there_is_more {
            let resp = rds_client
                .describe_db_instances()
                .set_marker(marker)
                .max_records(100)
                .send()
                .await
                .unwrap();
            marker = resp.marker().map(|m| m.to_owned());
            let instances = resp.db_instances();
            let rdses = instances.as_deref().unwrap();
            there_is_more = rdses.len() == 100;
            rdses.into_iter().for_each(|rds| {
                if let Some(_) = rds.db_name() {
                    let db_instance_arn = rds.db_instance_arn().unwrap().to_owned();
                    let name = name_regex
                        .captures(&db_instance_arn)
                        .and_then(|c| c.get(2))
                        .and_then(|c| Some(c.as_str().to_owned()))
                        .unwrap_or(db_instance_arn.split(":").last().unwrap().to_owned());
                    let tags = rds.tag_list().unwrap();
                    let mut appname_tag = String::from("");
                    let mut environment_tag = String::from("");
                    let endpoint = rds
                        .endpoint()
                        .map(|e| Endpoint {
                            address: e.address().unwrap().to_owned(),
                            port: u16::try_from(e.port()).unwrap(),
                        })
                        .unwrap()
                        .clone();
                    let mut env = Env::DEVNULL;
                    for t in tags {
                        if t.key().unwrap() == "AppName" {
                            appname_tag = t.value().unwrap().to_owned()
                        }
                        if t.key().unwrap() == "Environment" {
                            environment_tag = t.value().unwrap().to_owned();
                            env = Env::from_exact(&environment_tag);
                        }
                    }
                    let db = DbInstance {
                        name,
                        arn: db_instance_arn,
                        endpoint,
                        appname_tag,
                        environment_tag,
                        env: env.clone(),
                    };
                    self.databases.push(db)
                }
            });
        }
        self.databases.sort_by(|a, b| a.name.cmp(&b.name));
        self.databases.clone()
    }

    pub async fn db_secret(&self, name: &str, env: &Env) -> Result<DbSecret, BError> {
        let filter = secretsmanager::types::Filter::builder()
            .key("name".into())
            .values(&format!("{}-{}/db-credentials", name, env))
            .build();
        let secret_client = self.secret_client.as_ref().unwrap();
        let secret_arn = secret_client.list_secrets().filters(filter).send().await;

        let secret_arn = secret_arn.expect("Failed to fetch!");
        let secret_arn = secret_arn.secret_list().expect("No arn list!");
        if secret_arn.len() == 1 {
            let secret_arn = secret_arn.first().unwrap();
            let secret_arn = secret_arn.arn().expect("Expected arn password").clone();

            let secret = secret_client
                .get_secret_value()
                .secret_id(secret_arn.clone())
                .send()
                .await;
            if secret.is_err() {
                return Err(BError::new("db_secret", "No secrets found"));
            }
            let secret = secret.unwrap();
            let secret = secret.secret_string().expect("There should be a secret");
            let secret =
                serde_json::from_str::<DbSecretDTO>(secret).expect("Deserialzied DbSecret");

            return Ok(DbSecret {
                dbname: secret.dbname,
                password: secret.password,
                username: secret.username,
                auto_rotated: true,
            });
        } else {
            let param = self
                .ssm_client
                .as_ref()
                .unwrap()
                .get_parameter()
                .name(&format!("/config/{}_{}/datasource-password", name, env))
                .with_decryption(true)
                .send()
                .await;

            if let Ok(param) = param {
                if let Some(param) = param.parameter() {
                    return Ok(DbSecret {
                        dbname: name.to_owned(),
                        password: param.value().unwrap().to_owned(),
                        username: name.to_owned(),
                        auto_rotated: false,
                    });
                }
            }
            return Err(BError::new("db_secret", "No secrets found"));
        }
    }
}

pub struct EcsClient {
    ecs_client: Option<ecs::Client>,
    clusters: Vec<Cluster>,
    cluster_service_map: HashMap<Cluster, Vec<EcsService>>,
    service_details_map: HashMap<String, ServiceDetails>,
}

impl EcsClient {
    pub fn new() -> Self {
        EcsClient {
            ecs_client: None,
            clusters: Vec::new(),
            cluster_service_map: HashMap::new(),
            service_details_map: HashMap::new(),
        }
    }
    pub async fn init(&mut self, profile: &str) {
        self.ecs_client = Some(ecs_client(profile).await);
    }
    pub fn shutdown(&mut self) {
        self.ecs_client = None;
        self.clusters = Vec::new();
        self.clear();
    }

    pub fn clear(&mut self) {
        self.cluster_service_map = HashMap::new();
        self.service_details_map = HashMap::new();
    }

    pub async fn clusters(&mut self) -> Vec<Cluster> {
        println!("Getting clusters!");
        let ecs_client = self.ecs_client.as_ref().unwrap();
        if self.clusters.len() == 0 {
            let cluster_resp = &ecs_client
                .list_clusters()
                .send()
                .await
                .expect("Failed to get Cluster list");

            let cluster_arns = cluster_resp.cluster_arns().unwrap_or_default();

            for cluster_arn in cluster_arns {
                let env = Env::from_any(cluster_arn);
                self.clusters.push(Cluster {
                    name: shared::cluster_arn_to_name(cluster_arn),
                    arn: cluster_arn.clone(),
                    env: env,
                });
            }
        }
        self.clusters.clone()
    }
    pub async fn service_details(&mut self, service_arn: &str, refresh: bool) -> ServiceDetails {
        println!("Getting service details for {} {}", service_arn, refresh);
        let ecs_client = self.ecs_client.as_ref().unwrap();
        if refresh {
            self.service_details_map.remove(service_arn);
        }
        if let Some(details) = self.service_details_map.get(service_arn) {
            return details.clone();
        }
        let cluster = service_arn.split("/").collect::<Vec<&str>>()[1];
        let service = ecs_client
            .describe_services()
            .services(service_arn)
            .cluster(cluster)
            .send()
            .await
            .unwrap();
        let service = service.services().unwrap();
        let service = &service[0];
        let task_def_arn = service.task_definition().unwrap();
        let task_def = ecs_client
            .describe_task_definition()
            .task_definition(task_def_arn)
            .send()
            .await
            .unwrap();

        let task_def = task_def.task_definition().unwrap();
        let container_def = &task_def.container_definitions().unwrap()[0];
        let version = container_def
            .image()
            .unwrap()
            .split(":")
            .last()
            .unwrap()
            .to_owned();
        let details = ServiceDetails {
            name: shared::ecs_arn_to_name(&service_arn),
            timestamp: Utc::now(),
            arn: service_arn.to_owned(),
            cluster_arn: service.cluster_arn().unwrap().to_owned(),
            version: version,
            env: Env::from_any(&service_arn),
        };
        self.service_details_map
            .insert(service_arn.to_owned(), details.clone());

        return details;
    }

    pub async fn services(&mut self, cluster: &Cluster) -> Vec<EcsService> {
        println!("Getting services for {}", &cluster.arn);
        let ecs_client = self.ecs_client.as_ref().unwrap();

        if let Some(services) = self.cluster_service_map.get(cluster) {
            return services.clone();
        }

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
                .unwrap();
            next_token = services_resp.next_token().map(|t| t.to_owned());
            has_more = next_token.is_some();

            services_resp
                .service_arns()
                .unwrap()
                .iter()
                .for_each(|service_arn| {
                    values.push(EcsService {
                        name: service_arn.split("/").last().unwrap().to_owned(),
                        arn: service_arn.to_owned(),
                        cluster_arn: cluster.arn.to_owned(),
                        env: cluster.env.clone(),
                    })
                })
        }
        values.sort_by(|a, b| a.name.cmp(&b.name));
        self.cluster_service_map
            .insert(cluster.clone(), values.clone());
        values
    }

    pub async fn service_details_for_names(
        &mut self,
        names: &HashSet<String>,
        refresh: bool,
    ) -> HashMap<String, Vec<ServiceDetails>> {
        let mut result = HashMap::new();
        let service_arns: Vec<String> = self
            .cluster_service_map
            .values()
            .flatten()
            .filter(|s| names.contains(&s.name))
            .map(|s| s.arn.clone())
            .collect();

        for service_arn in service_arns {
            let sd = self.service_details(&service_arn, refresh).await;
            if !result.contains_key(&sd.name) {
                result.insert(sd.name.clone(), vec![sd]);
            } else {
                result.get_mut(&sd.name).unwrap().push(sd);
            }
        }
        result
    }
}
