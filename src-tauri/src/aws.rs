use std::fmt;

use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use aws_sdk_secretsmanager as secretsmanager;
use ec2::types::Filter;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::shared::BError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Env {
    DEVNULL,
    PLAY,
    LAB,
    DEV,
    DEMO,
    PROD,
}
impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Env::DEVNULL => write!(f, "devnull"),
            Env::PLAY => write!(f, "play"),
            Env::LAB => write!(f, "lab"),
            Env::DEV => write!(f, "dev"),
            Env::DEMO => write!(f, "demo"),
            Env::PROD => write!(f, "prod"),
        }
    }
}
impl Env {
    pub fn from_exact(str: &str) -> Env {
        match str {
            "play" => Env::PLAY,
            "lab" => Env::LAB,
            "dev" => Env::DEV,
            "demo" => Env::DEMO,
            "prod" => Env::PROD,
            _ => Env::DEVNULL,
        }
    }
    pub fn from_any(str: &str) -> Env {
        let env_regex = Regex::new(".*(play|lab|dev|demo|prod).*").unwrap();
        let captures = env_regex.captures(str);
        let env = captures
            .and_then(|c| c.get(1))
            .and_then(|e| Some(e.as_str().to_owned()))
            .unwrap_or("".to_owned());

        Env::from_exact(&env)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bastion {
    pub arn: String,
    pub instance_id: String,
    pub env: Env,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
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
pub struct DbSecret {
    dbInstanceIdentifier: String,
    pub dbname: String,
    engine: String,
    host: String,
    pub password: String,
    port: u16,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsService {
    pub name: String,
    pub arn: String,
    pub cluster_arn: String,
    pub env: Env,
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

pub async fn clusters(ecs: &ecs::Client) -> Vec<Cluster> {
    let mut clusters = vec![];
    let cluster_resp = ecs
        .list_clusters()
        .send()
        .await
        .expect("Failed to get Cluster list");

    let cluster_arns = cluster_resp.cluster_arns().unwrap_or_default();

    for cluster_arn in cluster_arns {
        let env = Env::from_any(cluster_arn);
        clusters.push(Cluster {
            arn: cluster_arn.clone(),
            env: env,
        });
    }
    clusters
}

pub async fn services(ecs: &ecs::Client, cluster: &Cluster) -> Vec<EcsService> {
    let mut values = vec![];
    let mut has_more = true;
    let mut next_token = None;

    while has_more {
        let services_resp = ecs
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
    values
}

pub async fn databases(rds: &rds::Client) -> Vec<DbInstance> {
    let mut databases = vec![];
    let mut there_is_more = true;
    let mut marker = None;
    let name_regex = Regex::new(".*(play|lab|dev|demo|prod)-(.*)").unwrap();
    while there_is_more {
        let resp = rds
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
                databases.push(db)
            }
        });
    }
    databases.sort_by(|a, b| a.name.cmp(&b.name));
    databases
}

pub async fn db_secret(
    client: &secretsmanager::Client,
    secret_identified: &str,
) -> Result<DbSecret, BError> {
    let filter = secretsmanager::types::Filter::builder()
        .key("name".into())
        .values(secret_identified)
        .build();
    let secret_arn = client.list_secrets().filters(filter).send().await;

    let secret_arn = secret_arn.expect("Failed to fetch!");
    let secret_arn = secret_arn.secret_list().expect("No arn list!");
    if secret_arn.len() == 1 {
        let secret_arn = secret_arn.first().unwrap();
        let secret_arn = secret_arn.arn().expect("Expected arn password").clone();

        let secret = client
            .get_secret_value()
            .secret_id(secret_arn.clone())
            .send()
            .await;
        if secret.is_err() {
            return Err(BError::new("db_secret", "No secrets found"));
        }
        let secret = secret.unwrap();
        let secret = secret.secret_string().expect("There should be a secret");
        let secret = serde_json::from_str::<DbSecret>(secret).expect("Deserialzied DbSecret");
        return Ok(secret);
    } else {
        return Err(BError::new("db_secret", "No secrets found"));
    }
}
