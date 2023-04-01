// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_rds as rds;
use ec2::types::Filter;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::HeaderValue;
use warp::hyper::body::Bytes;
use warp::hyper::Method;
use warp::Filter as WarpFilter;
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
async fn clusters(state: tauri::State<'_, GlobalThreadSafeState>) -> Result<Vec<Cluster>, BError> {
    let mut state = state.0.lock().await;
    return Ok(state.clusters().await);
}

#[tauri::command]
async fn services(
    env: Env,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<Vec<EcsService>, BError> {
    return Ok(state.0.lock().await.services(env).await);
}
#[tauri::command]
async fn databases(
    env: Env,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<Vec<DbInstance>, ()> {
    return Ok(state.0.lock().await.databases(env).await);
}

#[tauri::command]
async fn start_db_proxy(
    db: DbInstance,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<(), ()> {
    let mut state = state.0.lock().await;
    let bastion = state.bastion(db.env).await;
    let profile = state.user_config.clone().last_used_profile.unwrap();

    start_aws_ssm_proxy(
        bastion.instance_id,
        profile,
        db.endpoint.address,
        db.endpoint.port,
        8081,
    );
    Ok(())
}
#[tauri::command]
async fn open_dbeaver(
    db: DbInstance,
    state: tauri::State<'_, GlobalThreadSafeState>,
) -> Result<(), ()> {
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
fn start_aws_ssm_proxy(
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
        println!("THE END");
        dbg!(output);
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
            open_dbeaver,
            start_db_proxy
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
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DbInstance {
    name: String,
    endpoint: Endpoint,
    arn: String,
    environment_tag: String,
    env: Env,
    appname_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EcsService {
    name: String,
    arn: String,
    cluster_arn: String,
    env: Env,
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

    async fn databases(&mut self, env: Env) -> Vec<DbInstance> {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .databases(env)
            .await
    }

    async fn services(&mut self, env: Env) -> Vec<EcsService> {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .services(env)
            .await
    }
    async fn clusters(&mut self) -> Vec<Cluster> {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .clusters()
            .await
            .values()
            .cloned()
            .collect()
    }
    async fn bastion(&mut self, env: Env) -> Bastion {
        self.aws_client
            .as_mut()
            .expect("Login first!")
            .bastion(env)
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum Env {
    DEVNULL,
    PLAY,
    LAB,
    DEV,
    DEMO,
    PROD,
}

impl Env {
    fn from_exact(str: &str) -> Env {
        match str {
            "play" => Env::PLAY,
            "lab" => Env::LAB,
            "dev" => Env::DEV,
            "demo" => Env::DEMO,
            "prod" => Env::PROD,
            _ => Env::DEVNULL,
        }
    }
    fn from_any(str: &str) -> Env {
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
struct Bastion {
    arn: String,
    instance_id: String,
    env: Env,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cluster {
    arn: String,
    env: Env,
}

struct AwsCache {
    bastions: HashMap<Env, Bastion>,
    clusters: HashMap<Env, Cluster>,
    databases: Vec<DbInstance>,
    services: HashMap<Env, Vec<EcsService>>,
}

impl AwsCache {
    fn default() -> AwsCache {
        AwsCache {
            bastions: HashMap::new(),
            clusters: HashMap::new(),
            databases: Vec::new(),
            services: HashMap::new(),
        }
    }
}

struct AwsClient {
    cache: AwsCache,
    ecs: ecs::Client,
    rds: rds::Client,
    ec2: ec2::Client,
}

impl AwsClient {
    async fn is_logged(ecs: &ecs::Client) -> bool {
        let resp = ecs.list_clusters().send().await;
        return resp.is_ok();
    }

    async fn default(profile: &str) -> Result<AwsClient, String> {
        let config = Some(aws_config::from_env().profile_name(profile).load().await);
        let ecs = ecs::Client::new(config.as_ref().unwrap());
        let rds = rds::Client::new(config.as_ref().unwrap());
        let ec2 = ec2::Client::new(config.as_ref().unwrap());

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
            ec2,
        })
    }

    async fn bastion(&mut self, env: Env) -> Bastion {
        if self.cache.bastions.is_empty() {
            let filter = Filter::builder()
                .name("tag:Name")
                .values("*-bastion*")
                .build();
            let res = self.ec2.describe_instances().filters(filter).send().await;
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
            dbg!(&res);
            for b in res {
                self.cache.bastions.insert(b.env.clone(), b);
            }
            dbg!(&self.cache.bastions);
        }
        self.cache.bastions.get(&env).unwrap().clone()
    }

    async fn clusters(&mut self) -> HashMap<Env, Cluster> {
        if self.cache.clusters.is_empty() {
            let cluster_resp = self
                .ecs
                .list_clusters()
                .send()
                .await
                .expect("Failed to get Cluster list");

            let cluster_arns = cluster_resp.cluster_arns().unwrap_or_default();

            for cluster_arn in cluster_arns {
                let env = Env::from_any(cluster_arn);
                self.cache.clusters.insert(
                    env.clone(),
                    Cluster {
                        arn: cluster_arn.clone(),
                        env: env,
                    },
                );
            }
        }

        self.cache.clusters.clone()
    }

    async fn services(&mut self, env: Env) -> Vec<EcsService> {
        if !self.cache.services.contains_key(&env) {
            let mut values = vec![];
            let mut has_more = true;
            let mut next_token = None;
            if let Some(cluster) = self.clusters().await.get(&env) {
                while has_more {
                    let services_resp = self
                        .ecs
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
                                env: env.clone(),
                            })
                        })
                }
            }
            values.sort_by(|a, b| a.name.cmp(&b.name));
            self.cache.services.insert(env.clone(), values);
        }
        self.cache
            .services
            .get(&env.clone())
            .expect("Services cached was not filled")
            .clone()
    }

    async fn databases(&mut self, env: Env) -> Vec<DbInstance> {
        if self.cache.databases.is_empty() {
            let mut there_is_more = true;
            let mut marker = None;
            let name_regex = Regex::new(".*(play|lab|dev|demo|prod)-(.*)").unwrap();
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
                        self.cache.databases.push(db)
                    }
                });
            }
            self.cache.databases.sort_by(|a, b| a.name.cmp(&b.name))
        }

        self.cache
            .databases
            .iter()
            .filter(|db| db.env == env)
            .cloned()
            .collect()
    }
}
