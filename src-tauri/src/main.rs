// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws_config::SdkConfig;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

// ./dbeaver-cli.exe
//  -con "driver=mariadb|id=viproxy_cogitor|name=vprx_piotrcogitor_cogitor-ci|host=mariadb106.piotrcogitor.nazwa.pl|user=piotrcogitor_cogitor-ci|password=zpk7rjx4bqn6xec-XBN|openConsole=true|folder=viproxy|create=true|save=true"
// let output = Command::new("D:/repos/viproxy/proxy/proxy.exe")
//     .output()
//     .expect("ls command failed to start");
// https://docs.rs/hyper-reverse-proxy/latest/hyper_reverse_proxy/

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
// #[tauri::command]
// async fn clusters(state: tauri::State<'_, AwsState>) -> Result<Vec<(Environment, String)>, String> {
//     return Ok(state.clusters().await);
// }

#[tauri::command]
async fn set_environment(env: &str, state: tauri::State<'_, WombatState>) -> Result<(), ()> {
    state.0.lock().await.change_env(env);
    return Ok(());
}
#[tauri::command]
async fn records(state: tauri::State<'_, WombatState>) -> Result<Vec<Entry>, ()> {
    let res = state.0.lock().await.records();
    return Ok(res);
}
#[tauri::command]
async fn login(profile: &str, state: tauri::State<'_, WombatState>) -> Result<Vec<Entry>, String> {
    let init_result = state.0.lock().await.init(profile).await;
    return init_result;

    // return Ok(records(state).await.unwrap());
}
// #[tauri::command]
// fn open_db_connection(name: &str) -> String {
//     // ./dbeaver-cli.exe
//     //  -con "driver=mariadb|id=viproxy_cogitor|name=vprx_piotrcogitor_cogitor-ci|host=mariadb106.piotrcogitor.nazwa.pl|user=piotrcogitor_cogitor-ci|password=zpk7rjx4bqn6xec-XBN|openConsole=true|folder=viproxy|create=true|save=true"
//     // let output = Command::new("D:/repos/viproxy/proxy/proxy.exe")
//     //     .output()
//     //     .expect("ls command failed to start");
//     format!("Hello, {}! You've been greeted from Rust!", name)
// }

#[tokio::main]
async fn main() {
    fix_path_env::fix().unwrap();
    let state = AwsState::default().await;
    let managed_state = WombatState(Arc::new(Mutex::new(state)));

    tauri::Builder::default()
        .manage(managed_state)
        .invoke_handler(tauri::generate_handler![login, set_environment, records])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
enum Environment {
    PLAY,
    LAB,
    DEV,
    DEMO,
    PROD,
    UNKNOWN,
}

impl FromStr for Environment {
    type Err = ();
    fn from_str(input: &str) -> Result<Environment, Self::Err> {
        match input {
            "PLAY" => Ok(Environment::PLAY),
            "LAB" => Ok(Environment::LAB),
            "DEV" => Ok(Environment::DEV),
            "DEMO" => Ok(Environment::DEMO),
            "PROD" => Ok(Environment::PROD),
            "UNKNOWN" => Ok(Environment::UNKNOWN),
            _ => Err(()),
        }
    }
}

struct WombatState(Arc<Mutex<AwsState>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry {
    service: String,
    service_arn: String,
    dbs: Vec<DbInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Endpoint {
    address: String,
    port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DbInstance {
    db_name: String,
    endpoint: Endpoint,
    db_instance_arn: String,
    env: Environment,
    service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EcsService {
    name: String,
    service_arn: String,
    env: Environment,
}

struct AwsState {
    profile: String,
    config: Option<SdkConfig>,
    ecs: Option<ecs::Client>,
    rds: Option<rds::Client>,
    env: Environment,
    clusters: HashMap<Environment, String>,
    databases: Vec<DbInstance>,
    services: Vec<EcsService>,
}

impl AwsState {
    async fn default() -> AwsState {
        let profile = "developer".to_owned();

        AwsState {
            profile,
            config: None,
            ecs: None,
            rds: None,
            env: Environment::DEV,
            clusters: HashMap::new(),
            databases: Vec::new(),
            services: Vec::new(),
        }
    }

    fn change_env(&mut self, env: &str) {
        self.env = Environment::from_str(env).unwrap()
    }

    fn records(&self) -> Vec<Entry> {
        return self
            .services
            .iter()
            .filter(|s| s.env == self.env)
            .map(|s| {
                let dbs = self
                    .databases
                    .iter()
                    .filter(|db| db.service == s.name && db.env == s.env)
                    .map(|db| db.to_owned())
                    .collect();
                Entry {
                    service: s.name.to_owned(),
                    service_arn: s.service_arn.to_owned(),
                    dbs: dbs,
                }
            })
            .collect();
    }

    async fn is_logged(&self) -> bool {
        return self
            .ecs
            .as_ref()
            .unwrap()
            .list_clusters()
            .send()
            .await
            .is_ok();
    }

    async fn init(&mut self, profile: &str) -> Result<Vec<Entry>, String> {
        self.profile = profile.to_owned();
        self.config = Some(aws_config::from_env().profile_name(profile).load().await);
        self.ecs = Some(ecs::Client::new(self.config.as_ref().unwrap()));
        self.rds = Some(rds::Client::new(self.config.as_ref().unwrap()));
        self.clusters = HashMap::new();
        self.databases = Vec::new();
        self.services = Vec::new();
        if !self.is_logged().await {
            println!("Trigger log in into AWS");
            Command::new("aws")
                .args(["sso", "login", "--profile", &self.profile])
                .output()
                .expect("failed to execute process");
        }
        if !self.is_logged().await {
            return Err(String::from("Failed to log in."));
        }
        self.refresh_clusters().await;
        self.refresh_services().await;
        self.refresh_rds().await;
        Ok(self.records())
    }

    async fn refresh_clusters(&mut self) {
        let ecs_client = self.ecs.as_ref().unwrap();
        let resp = ecs_client.list_clusters().send().await.unwrap();

        let cluster_arns = resp.cluster_arns().unwrap_or_default();
        println!("Found {} clusters:", cluster_arns.len());

        let clusters = ecs_client
            .describe_clusters()
            .set_clusters(Some(cluster_arns.into()))
            .send()
            .await
            .unwrap();
        let env_regex = Regex::new("(play|lab|dev|demo|prod)").unwrap();
        clusters.clusters().unwrap().iter().for_each(|cluster| {
            let arn = cluster.cluster_arn().unwrap().to_owned();
            let captures = env_regex.captures(&arn);
            if let Some(e) = captures {
                let env_enum =
                    Environment::from_str(&e.get(0).unwrap().as_str().to_uppercase()).unwrap();
                println!("Adding {:?}: {}", &env_enum, &arn);
                self.clusters.insert(env_enum, arn);
            } else {
                println!("Skipping: {}", &arn)
            }
        })
    }

    async fn refresh_rds(&mut self) {
        if self.databases.len() == 0 {
            let mut there_is_more = true;
            let mut marker = None;
            while there_is_more {
                let resp = self
                    .rds
                    .as_ref()
                    .unwrap()
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
                        let db_name = db_instance_arn.split(":").last().unwrap().to_owned();
                        let tags = rds.tag_list().unwrap();
                        let mut service = String::from("");
                        let mut env = Environment::LAB;
                        let endpoint = rds
                            .endpoint()
                            .map(|e| Endpoint {
                                address: e.address().unwrap().to_owned(),
                                port: e.port(),
                            })
                            .unwrap()
                            .clone();
                        for t in tags {
                            if t.key().unwrap() == "AppName" {
                                service = t.value().unwrap().to_owned()
                            }
                            if t.key().unwrap() == "Environment" {
                                env = Environment::from_str(&t.value().unwrap().to_uppercase())
                                    .unwrap_or(Environment::UNKNOWN)
                            }
                        }
                        // let service =  .get("AppName").unwrap();
                        // let service =  rds.tag_list().unwrap().get("AppName").unwrap();
                        println!("DB {}: {:?}", &db_instance_arn, &env);
                        self.databases.push(DbInstance {
                            env,
                            service,
                            db_name,
                            db_instance_arn,
                            endpoint,
                        })
                    }
                });
            }
        }
    }

    async fn refresh_services(&mut self) {
        for cluster in self.clusters.iter() {
            let cluster_arn = cluster.1;
            let mut has_more = true;
            let mut next_token = None;
            while has_more {
                let resp = self
                    .ecs
                    .as_ref()
                    .unwrap()
                    .list_services()
                    .set_cluster(Option::Some(cluster_arn.to_owned()))
                    .max_results(100)
                    .set_next_token(next_token)
                    .send()
                    .await
                    .unwrap();
                next_token = resp.next_token().map(|t| t.to_owned());
                has_more = next_token.is_some();
                resp.service_arns()
                    .as_deref()
                    .unwrap()
                    .iter()
                    .for_each(|service_arn| {
                        let ecs_instance = EcsService {
                            name: service_arn.split("/").last().unwrap().to_owned(),
                            service_arn: service_arn.to_owned(),
                            env: cluster.0.to_owned(),
                        };
                        println!("ECS {}", &ecs_instance.name);

                        self.services.push(ecs_instance);
                    })
            }
        }
    }
}
