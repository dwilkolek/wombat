// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws_config::SdkConfig;
use aws_sdk_ecs as ecs;
use ecs::Client;
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use tauri::regex::Regex;
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
async fn get_services(state: tauri::State<'_, WombatState>) -> Result<Vec<String>, ()> {
    let res = state.0.lock().await.services().await;
    return Ok(res);
}

#[tauri::command]
async fn get_rds(state: tauri::State<'_, WombatState>) -> Result<(), ()> {
    return Ok(());
}

#[tauri::command]
fn open_db_connection(name: &str) -> String {
    // ./dbeaver-cli.exe
    //  -con "driver=mariadb|id=viproxy_cogitor|name=vprx_piotrcogitor_cogitor-ci|host=mariadb106.piotrcogitor.nazwa.pl|user=piotrcogitor_cogitor-ci|password=zpk7rjx4bqn6xec-XBN|openConsole=true|folder=viproxy|create=true|save=true"
    // let output = Command::new("D:/repos/viproxy/proxy/proxy.exe")
    //     .output()
    //     .expect("ls command failed to start");
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tokio::main]
async fn main() {
    let mut state = AwsState::default().await;
    state.init().await;
    let managed_state = WombatState(Arc::new(Mutex::new(state)));

    tauri::Builder::default()
        .manage(managed_state)
        .invoke_handler(tauri::generate_handler![
            set_environment,
            get_services,
            get_rds
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Environment {
    LAB,
    DEV,
    DEMO,
    PROD,
}

impl FromStr for Environment {
    type Err = ();
    fn from_str(input: &str) -> Result<Environment, Self::Err> {
        match input {
            "LAB" => Ok(Environment::LAB),
            "DEV" => Ok(Environment::DEV),
            "DEMO" => Ok(Environment::DEMO),
            "PROD" => Ok(Environment::PROD),
            _ => Err(()),
        }
    }
}

struct WombatState(Arc<Mutex<AwsState>>);

struct AwsState {
    profile: String,
    config: SdkConfig,
    client: Client,
    env: Environment,
    clusters: HashMap<Environment, String>,
}

impl AwsState {
    async fn default() -> AwsState {
        let profile = "developer".to_owned();
        let config = aws_config::from_env().profile_name(&profile).load().await;
        let client = ecs::Client::new(&config);

        AwsState {
            profile,
            config,
            client,
            env: Environment::DEV,
            clusters: HashMap::new(),
        }
    }

    fn change_env(&mut self, env: &str) {
        self.env = Environment::from_str(env).unwrap()
    }

    async fn init(&mut self) {
        if self.client.list_clusters().send().await.is_err() {
            println!("Loggin to AWS");
            Command::new("aws")
                .args(["sso", "login", "--profile", &self.profile])
                .output()
                .expect("failed to execute process");
        }
        self.refresh_clusters().await
    }

    async fn refresh_clusters(&mut self) {
        let resp = self.client.list_clusters().send().await.unwrap();

        let cluster_arns = resp.cluster_arns().unwrap_or_default();
        println!("Found {} clusters:", cluster_arns.len());

        let clusters = self
            .client
            .describe_clusters()
            .set_clusters(Some(cluster_arns.into()))
            .send()
            .await
            .unwrap();
        let env_regex = Regex::new("(lab|dev|demo|prod)").unwrap();
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

    async fn services(&self) -> Vec<String> {
        let cluster_arn = self.clusters.get(&self.env).unwrap();
        let resp = self
            .client
            .list_services()
            .set_cluster(Option::Some(cluster_arn.to_owned()))
            .send()
            .await
            .unwrap();

        resp.service_arns().as_deref().unwrap().to_vec()
    }
}
