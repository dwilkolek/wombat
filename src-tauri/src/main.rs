// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::HeaderValue;
use warp::hyper::body::Bytes;
use warp::hyper::Method;
use warp::Filter;
use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response, Headers};

#[tauri::command]
async fn user_config(state: tauri::State<'_, GlobalThreadSafeState>) -> Result<UserConfig, BError> {
    return Ok(state.0.lock().await.user_config.clone());
}

#[tauri::command]
async fn toggle_service_favourite(
    service_name: &str,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<UserConfig, BError> {
    return state.0.lock().await.toggle_service_favourite(service_name);
}
#[tauri::command]
async fn toggle_db_favourite(
    db_arn: &str,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<UserConfig, BError> {
    return state.0.lock().await.toggle_db_favourite(db_arn);
}

#[tauri::command]
async fn login(
    profile: &str,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<UserConfig, BError> {
    return state.0.lock().await.login(profile).await;
}

#[tauri::command]
async fn set_dbeaver_path(
    dbeaver_path: &str,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<UserConfig, BError> {
    return state.0.lock().await.set_dbeaver_path(dbeaver_path);
}

#[tauri::command]
async fn logout(state: tauri::State<'_, GlobalThreadSafeState>) -> Result<(), BError> {
    state.0.lock().await.logout();
    Ok(())
}
#[tauri::command]
async fn clusters(state: tauri::State<'_, GlobalThreadSafeState>) -> Result<Vec<String>, BError> {
    return Ok(state.0.lock().await.clusters().await);
}

#[tauri::command]
async fn services(
    cluster_arn: &str,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<Vec<EcsService>, BError> {
    return Ok(state.0.lock().await.services(cluster_arn).await);
}
#[tauri::command]
async fn databases(state: tauri::State<'_, GlobalThreadSafeState>) -> Result<Vec<DbInstance>, ()> {
    return Ok(state.0.lock().await.databases().await);
}

#[tauri::command]
async fn open_dbeaver(state: tauri::State<'_, GlobalThreadSafeState>) -> Result<(), ()> {
    // fn db_beaver_con_parma(arn: &str, db_name: &str, host: &str, user: &str, password: &str) -> String {
    //     format!(
    //         "driver=postgres|id={}|name={}|host={}|user={}|password={}|openConsole=true|folder=wombat|create=true|save=true",
    //         arn, db_name, host, user, password
    //     )
    // }
    // let app_state = state.0.lock().await;
    // let dbeaver_path = &app_state
    //     .user_config
    //     .dbeaver_path
    //     .as_ref()
    //     .expect("DBeaver needs to be configured")
    //     .clone();

    // Command::new(dbeaver_path)
    //     .args(["-con", &db_beaver_con_parma("", "", "", "", "")])
    //     .output()
    //     .expect("failed to execute process");
    // return Ok(());
    todo!()
}

#[tauri::command]
async fn start_local_proxy(
    local_port: i32,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<(), ()> {
    // let request_filter = extract_request_data_filter();
    // let host = "";
    // let aws_port: u16 = 10000;
    // let app = warp::any().and(request_filter).and_then(
    //     move |uri: warp::path::FullPath,
    //           params: Option<String>,
    //           method: Method,
    //           mut headers: Headers,
    //           body: Bytes| {
    //         headers.insert("Origin", host.parse().unwrap());
    //         headers.insert("Host", host.parse().unwrap());
    //         proxy_to_and_forward_response(
    //             format!("http://localhost:{}/", aws_port).to_owned(),
    //             "".to_owned(),
    //             uri,
    //             params,
    //             method,
    //             headers,
    //             body,
    //         )
    //     },
    // );

    // // spawn proxy server
    // warp::serve(app).run(([0, 0, 0, 0], aws_port)).await;
    todo!()
}

//untested
async fn start_proxy_to_aws_proxy(service_header: Option<String>, aws_port: u16) {
    tokio::task::spawn(async move {
        let request_filter = extract_request_data_filter();
        let header_value = service_header
            .unwrap_or(String::from(""))
            .parse::<HeaderValue>()
            .unwrap();
        let app = warp::any().and(request_filter).and_then(
            move |uri: warp::path::FullPath,
                  params: Option<String>,
                  method: Method,
                  mut headers: Headers,
                  body: Bytes| {
                // if let Some(header_value) = service_header {
                headers.insert("Origin", header_value.clone());
                headers.insert("Host", header_value.clone());
                // }

                proxy_to_and_forward_response(
                    format!("http://localhost:{}/", aws_port).to_owned(),
                    "".to_owned(),
                    uri,
                    params,
                    method,
                    headers,
                    body,
                )
            },
        );
        warp::serve(app).run(([0, 0, 0, 0], aws_port)).await;
    });
}

//untested
async fn start_aws_ssm_proxy(
    bastion: String,
    profile: String,
    target: String,
    target_port: u16,
    local_port: u16,
) {
    tokio::task::spawn(async move {
        // {\"host\":[\"$endpoint\"], \"portNumber\":[\"5432\"], \"localPortNumber\":[\"$port\"]}
        // aws ssm start-session \
        //  --target "$instance" \
        //  --profile "$profile" \
        //  --document-name AWS-StartPortForwardingSessionToRemoteHost \
        //  --parameters "$parameters"
        let output = Command::new("aws")
            .args([
                "ssm",
                "start-session",
                "--target",
                &bastion,
                "--profile",
                &profile,
                "--document-name",
                "AWS-StartPortForwardingSessionToRemoteHost",
                "--parameters",
                &format!(
                    "{{\"host\":[\"{}\"], \"portNumber\":[\"{}\"], \"localPortNumber\":[\"{}\"]}}",
                    target, target_port, local_port
                ),
            ])
            .output()
            .expect("failed to execute process");
    });
}

#[tokio::main]
async fn main() {
    fix_path_env::fix().unwrap();

    let state = AppState::default().await;
    let managed_state = GlobalThreadSafeState(Arc::new(Mutex::new(state)));

    tauri::Builder::default()
        .manage(managed_state)
        .invoke_handler(tauri::generate_handler![
            user_config,
            login,
            logout,
            clusters,
            services,
            databases,
            set_dbeaver_path,
            toggle_service_favourite,
            toggle_db_favourite,
            open_dbeaver
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BError {
    message: String,
    command: String,
}
impl BError {
    fn new(command: &str, message: impl Into<String>) -> BError {
        BError {
            command: message.into(),
            message: command.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonitoringConfig {
    service_arn: Option<String>,
    database_arn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserConfig {
    last_used_profile: Option<String>,
    known_profiles: HashSet<String>,
    favourite_service_names: HashSet<String>,
    favourite_db_arns: HashSet<String>,
    dbeaver_path: Option<String>,
}

impl UserConfig {
    fn default() -> UserConfig {
        let config_file = UserConfig::config_path();
        let user_config = match std::fs::read_to_string(config_file) {
            Ok(json) => serde_json::from_str::<UserConfig>(&json).unwrap(),
            Err(_) => UserConfig {
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

    fn set_dbeaver_path(&mut self, dbeaver_path: &str) -> Result<UserConfig, BError> {
        if std::path::Path::new(dbeaver_path).exists() {
            self.dbeaver_path = Some(dbeaver_path.to_owned());
            self.save();
            Ok(self.clone())
        } else {
            Err(BError::new("set_dbeaver_path", "Invalid path!"))
        }
    }

    fn use_profile(&mut self, profile: &str) {
        self.last_used_profile = Some(profile.to_owned());
        self.known_profiles.insert(profile.to_owned());
        self.save()
    }

    fn toggle_service_favourite(&mut self, service_name: &str) -> Result<UserConfig, BError> {
        if !self
            .favourite_service_names
            .remove(&service_name.to_owned())
        {
            self.favourite_service_names.insert(service_name.to_owned());
        }

        self.save();
        Ok(self.clone())
    }

    fn toggle_db_favourite(&mut self, db_arn: &str) -> Result<UserConfig, BError> {
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

struct GlobalThreadSafeState(Arc<Mutex<AppState>>);

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
    environment_tag: String,
    appname_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EcsService {
    name: String,
    service_arn: String,
    cluster_arn: String,
}

struct AppState {
    active_profile: Option<String>,
    user_config: UserConfig,
    aws_client: Option<AwsClient>,
}

impl AppState {
    async fn default() -> AppState {
        let user_config = UserConfig::default();
        let aws_client = None;
        AppState {
            active_profile: user_config.last_used_profile.clone(),
            user_config,
            aws_client,
        }
    }

    fn logout(&mut self) {
        self.aws_client = None;
    }

    fn set_dbeaver_path(&mut self, dbeaver_path: &str) -> Result<UserConfig, BError> {
        self.user_config.set_dbeaver_path(dbeaver_path)
    }

    fn toggle_service_favourite(&mut self, service_name: &str) -> Result<UserConfig, BError> {
        self.user_config.toggle_service_favourite(service_name)
    }
    fn toggle_db_favourite(&mut self, db_arn: &str) -> Result<UserConfig, BError> {
        self.user_config.toggle_db_favourite(db_arn)
    }

    async fn login(&mut self, profile: &str) -> Result<UserConfig, BError> {
        self.user_config.use_profile(profile);
        self.active_profile = Some(profile.to_owned());
        let client_result = AwsClient::default(
            self.active_profile
                .to_owned()
                .expect("Active profile required")
                .as_str(),
        )
        .await;

        match client_result {
            Ok(client) => {
                self.aws_client = Some(client);
                Ok(self.user_config.clone())
            }
            Err(message) => Err(BError::new("login", message)),
        }
    }

    async fn databases(&mut self) -> Vec<DbInstance> {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .databases()
            .await
    }

    async fn services(&mut self, cluster_arn: &str) -> Vec<EcsService> {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .services(cluster_arn)
            .await
    }
    async fn clusters(&mut self) -> Vec<String> {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .clusters()
            .await
    }
}

struct AwsCache {
    clusters: Option<Vec<String>>,
    databases: Option<Vec<DbInstance>>,
    services: HashMap<String, Vec<EcsService>>,
}

impl AwsCache {
    fn default() -> AwsCache {
        AwsCache {
            clusters: None,
            databases: None,
            services: HashMap::new(),
        }
    }
}

struct AwsClient {
    cache: AwsCache,
    ecs: ecs::Client,
    rds: rds::Client,
}

impl AwsClient {
    async fn is_logged(ecs: &ecs::Client) -> bool {
        let resp = ecs.list_clusters().send().await;
        dbg!(&resp);
        return resp.is_ok();
    }

    async fn default(profile: &str) -> Result<AwsClient, String> {
        let config = Some(aws_config::from_env().profile_name(profile).load().await);
        let ecs = ecs::Client::new(config.as_ref().unwrap());
        let rds = rds::Client::new(config.as_ref().unwrap());

        if !AwsClient::is_logged(&ecs).await {
            println!("Trigger log in into AWS");
            let output = Command::new("aws")
                .args(["sso", "login", "--profile", &profile])
                .output()
                .expect("failed to execute process");
        }
        if !AwsClient::is_logged(&ecs).await {
            return Err(String::from("Failed to log it!"));
        }
        Ok(AwsClient {
            cache: AwsCache::default(),
            ecs,
            rds,
        })
    }

    async fn clusters(&mut self) -> Vec<String> {
        match &self.cache.clusters {
            Some(clusters) => clusters.clone(),
            None => {
                let cluster_resp = self
                    .ecs
                    .list_clusters()
                    .send()
                    .await
                    .expect("Failed to get cluser list");

                let cluster_arns = cluster_resp.cluster_arns().unwrap_or_default();
                self.cache.clusters = Some(cluster_arns.to_vec());
                cluster_arns.to_vec()
            }
        }
    }

    async fn services(&mut self, cluster_arn: &str) -> Vec<EcsService> {
        if !self.cache.services.contains_key(cluster_arn) {
            println!("Resolving services for {}", &cluster_arn);
            let mut values = vec![];
            let mut has_more = true;
            let mut next_token = None;
            while has_more {
                let services_resp = self
                    .ecs
                    .list_services()
                    .cluster(cluster_arn.to_owned())
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
                            service_arn: service_arn.to_owned(),
                            cluster_arn: cluster_arn.to_owned(),
                        })
                    })
            }
            values.sort_by(|a, b| a.name.cmp(&b.name));
            self.cache.services.insert(cluster_arn.to_owned(), values);
        }
        self.cache
            .services
            .get(cluster_arn)
            .expect("Services cached was not filled")
            .clone()
    }

    async fn databases(&mut self) -> Vec<DbInstance> {
        match &self.cache.databases {
            Some(databases) => databases.clone(),
            None => {
                let mut databases: Vec<DbInstance> = vec![];
                let mut there_is_more = true;
                let mut marker = None;
                while there_is_more {
                    let resp = self
                        .rds
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
                            let mut appname_tag = String::from("");
                            let mut environment_tag = String::from("");
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
                                    appname_tag = t.value().unwrap().to_owned()
                                }
                                if t.key().unwrap() == "Environment" {
                                    environment_tag = t.value().unwrap().to_owned()
                                }
                            }
                            databases.push(DbInstance {
                                db_name,
                                db_instance_arn,
                                endpoint,
                                appname_tag,
                                environment_tag,
                            })
                        }
                    });
                }
                databases.sort_by(|a, b| a.db_name.cmp(&b.db_name));
                self.cache.databases = Some(databases.clone());
                databases.clone()
            }
        }
    }
}
