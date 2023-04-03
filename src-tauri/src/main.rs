// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws::{ecs_client, Cluster};
use shared::BError;
use std::sync::Arc;
use std::{collections::HashMap, process::Command};
use tokio::sync::Mutex;
use user::UserConfig;
use warp::http::HeaderValue;
use warp::hyper::body::Bytes;
use warp::hyper::Method;
use warp::Filter as WarpFilter;
use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response, Headers};

mod aws;
mod shared;
mod user;

#[tauri::command]
async fn user_config(user_config: tauri::State<'_, UserConfigState>) -> Result<UserConfig, BError> {
    let user_config = user_config.0.lock().await;
    Ok(user_config.clone())
}

#[tauri::command]
async fn toggle_service_favourite(
    service_name: &str,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    user_config.toggle_service_favourite(service_name);
    Ok(user_config.clone())
}

#[tauri::command]
async fn toggle_db_favourite(
    db_arn: &str,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    user_config.toggle_db_favourite(db_arn);
    Ok(user_config.clone())
}

#[tauri::command]
async fn login(
    profile: &str,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let ecs_client = aws::ecs_client(profile).await;
    if !aws::is_logged(&ecs_client).await {
        println!("Trigger log in into AWS");
        let output = Command::new("aws")
            .args(["sso", "login", "--profile", &profile])
            .output()
            .expect("failed to execute process");
    }
    if !aws::is_logged(&ecs_client).await {
        return Err(BError::new("login", "Failed to log in"));
    }
    let mut user_config = user_config.0.lock().await;
    user_config.use_profile(profile);
    app_state.0.lock().await.active_profile = Some(profile.to_owned());
    Ok(user_config.clone())
}

#[tauri::command]
async fn set_dbeaver_path(
    dbeaver_path: &str,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    user_config.set_dbeaver_path(dbeaver_path)
}

#[tauri::command]
async fn logout(
    app_state: tauri::State<'_, AppContextState>,
    cluster_cache: tauri::State<'_, ClustersCache>,
    service_cache: tauri::State<'_, ServicesCache>,
    db_cache: tauri::State<'_, DatabasesCache>,
) -> Result<(), BError> {
    let mut app_state = app_state.0.lock().await;
    app_state.active_profile = None;
    cluster_cache.0.lock().await.clear();
    service_cache.0.lock().await.clear();
    db_cache.0.lock().await.clear();
    Ok(())
}

#[tauri::command]
async fn clusters(
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, ClustersCache>,
) -> Result<Vec<aws::Cluster>, BError> {
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };
    if let Some(active_profile) = active_profile {
        let mut cache = cache.0.lock().await;
        if cache.is_empty() {
            let clusters = aws::clusters(&ecs_client(&active_profile).await).await;

            for cluser in clusters.clone() {
                cache.push(cluser);
            }
        }

        Ok(cache.clone())
    } else {
        Err(BError::new("clusters", "Login required"))
    }
}

#[tauri::command]
async fn services(
    cluster: Cluster,
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, ServicesCache>,
) -> Result<Vec<aws::EcsService>, BError> {
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };
    if let Some(active_profile) = active_profile {
        let mut cache = cache.0.lock().await;
        if cache.get(&cluster.env).is_none() {
            let services = aws::services(&ecs_client(&active_profile).await, &cluster).await;
            cache.insert(cluster.env.clone(), services);
        }

        Ok(cache.get(&cluster.env).unwrap().clone())
    } else {
        Err(BError::new("services", "Login required"))
    }
}
#[tauri::command]
async fn databases(
    env: aws::Env,
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, DatabasesCache>,
) -> Result<Vec<aws::DbInstance>, BError> {
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };
    if let Some(active_profile) = active_profile {
        let mut cache = cache.0.lock().await;
        if cache.is_empty() {
            let databases = aws::databases(&aws::rds_client(&active_profile).await).await;
            for db in databases {
                cache.push(db);
            }
        }

        Ok(cache.iter().filter(|db| db.env == env).cloned().collect())
    } else {
        Err(BError::new("databases", "Login required"))
    }
}

// #[tauri::command]
// async fn start_db_proxy(
//     db: DbInstance,
//     state: tauri::State<'_, GlobalThreadSafeState>,
// ) -> Result<(), ()> {
//     let mut state = state.0.lock().await;
//     let bastion = state.bastion(db.env).await;
//     let profile = state.user_config.clone().last_used_profile.unwrap();

//     start_aws_ssm_proxy(
//         bastion.instance_id,
//         profile,
//         db.endpoint.address,
//         db.endpoint.port,
//         8081,
//     );
//     Ok(())
// }
// #[tauri::command]
// async fn open_dbeaver(
//     db: DbInstance,
//     state: tauri::State<'_, GlobalThreadSafeState>,
// ) -> Result<(), ()> {
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
//     todo!()
// }

// #[tauri::command]
// async fn start_local_proxy(
//     local_port: i32,
//     state: tauri::State<'_, GlobalThreadSafeState>,
// ) -> Result<(), ()> {
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
//     todo!()
// }

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

    tauri::Builder::default()
        .manage(UserConfigState(Arc::new(Mutex::new(UserConfig::default()))))
        .manage(AppContextState::default())
        .manage(BastionsCache::default())
        .manage(ClustersCache::default())
        .manage(DatabasesCache::default())
        .manage(ServicesCache::default())
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
            // open_dbeaver,
            // start_db_proxy
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// struct GlobalThreadSafeState(Arc<Mutex<AppState>>);

// struct AppState {
//     active_profile: Option<String>,
//     user_config: UserConfig,
//     aws_client: Option<AwsClient>,
// }

// impl AppState {
//     async fn default() -> AppState {
//         let user_config = UserConfig::default();
//         let aws_client = None;
//         AppState {
//             active_profile: user_config.last_used_profile.clone(),
//             user_config,
//             aws_client,
//         }
//     }

//     fn logout(&mut self) {
//         self.aws_client = None;
//     }

//     fn set_dbeaver_path(&mut self, dbeaver_path: &str) -> Result<UserConfig, BError> {
//         self.user_config.set_dbeaver_path(dbeaver_path)
//     }

//     fn toggle_service_favourite(&mut self, service_name: &str) -> Result<UserConfig, BError> {
//         self.user_config.toggle_service_favourite(service_name)
//     }
//     fn toggle_db_favourite(&mut self, db_arn: &str) -> Result<UserConfig, BError> {
//         self.user_config.toggle_db_favourite(db_arn)
//     }

//     async fn login(&mut self, profile: &str) -> Result<UserConfig, BError> {
//         self.user_config.use_profile(profile);
//         self.active_profile = Some(profile.to_owned());
//         let client_result = AwsClient::default(
//             self.active_profile
//                 .to_owned()
//                 .expect("Active profile required")
//                 .as_str(),
//         )
//         .await;

//         match client_result {
//             Ok(client) => {
//                 self.aws_client = Some(client);
//                 Ok(self.user_config.clone())
//             }
//             Err(message) => Err(BError::new("login", message)),
//         }
//     }

//     async fn databases(&mut self, env: Env) -> Vec<DbInstance> {
//         self.aws_client
//             .as_mut()
//             .expect("Login first!")
//             .databases(env)
//             .await
//     }

//     async fn services(&mut self, env: Env) -> Vec<EcsService> {
//         self.aws_client
//             .as_mut()
//             .expect("Login first!")
//             .services(env)
//             .await
//     }
//     async fn clusters(&mut self) -> Vec<Cluster> {
//         self.aws_client
//             .as_mut()
//             .expect("Login first!")
//             .clusters()
//             .await
//             .values()
//             .cloned()
//             .collect()
//     }
//     async fn bastion(&mut self, env: Env) -> Bastion {
//         self.aws_client
//             .as_mut()
//             .expect("Login first!")
//             .bastion(env)
//             .await
//     }
// }

#[derive(Default)]
struct BastionsCache(Arc<Mutex<Vec<aws::Bastion>>>);
#[derive(Default)]
struct ClustersCache(Arc<Mutex<Vec<aws::Cluster>>>);
#[derive(Default)]
struct DatabasesCache(Arc<Mutex<Vec<aws::DbInstance>>>);
#[derive(Default)]
struct ServicesCache(Arc<Mutex<HashMap<aws::Env, Vec<aws::EcsService>>>>);

#[derive(Default)]
struct AppContext {
    active_profile: Option<String>,
}

#[derive(Default)]
struct AppContextState(Arc<Mutex<AppContext>>);

struct UserConfigState(Arc<Mutex<UserConfig>>);
