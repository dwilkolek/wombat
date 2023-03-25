// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aws;

use std::{process::Command, sync::Mutex};

use aws_config::SdkConfig;
use aws_sdk_ecs as ecs;
use ecs::Client;

// ./dbeaver-cli.exe
//  -con "driver=mariadb|id=viproxy_cogitor|name=vprx_piotrcogitor_cogitor-ci|host=mariadb106.piotrcogitor.nazwa.pl|user=piotrcogitor_cogitor-ci|password=zpk7rjx4bqn6xec-XBN|openConsole=true|folder=viproxy|create=true|save=true"
// let output = Command::new("D:/repos/viproxy/proxy/proxy.exe")
//     .output()
//     .expect("ls command failed to start");
// https://docs.rs/hyper-reverse-proxy/latest/hyper_reverse_proxy/

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
async fn clusters(state: tauri::State<'_, AwsState>) -> Result<Vec<(String, String)>, String> {
    return Ok(state.clusters().await);
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
    let state = AwsState::default().await;
    state.ensure_logged_in().await;

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![clusters])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct AwsState {
    profile: String,
    config: SdkConfig,
    client: Client,
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
        }
    }

    async fn ensure_logged_in(&self) {
        if self.client.list_clusters().send().await.is_err() {
            println!("Loggin to AWS");
            Command::new("aws")
                .args(["sso", "login", "--profile", &self.profile])
                .output()
                .expect("failed to execute process");
        }
    }

    async fn clusters(&self) -> Vec<(String, String)> {
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

        // for cluster in clusters.clusters().unwrap_or_default() {
        //     println!("  ARN:  {}", cluster.cluster_arn().unwrap());
        //     println!("  Name: {}", cluster.cluster_name().unwrap());
        // }
        clusters
            .clusters()
            .unwrap()
            .iter()
            .map(|cluster| {
                (
                    cluster.cluster_arn().unwrap().to_owned(),
                    cluster.cluster_name().unwrap().to_owned(),
                )
            })
            .collect::<Vec<(String, String)>>()
    }
}
