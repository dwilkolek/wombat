// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws::{ecs_client, Cluster};
use shared::BError;
use std::sync::Arc;
use std::{collections::HashMap, process::Command};
use tauri::Window;
use tokio::sync::Mutex;
use urlencoding::encode;
use user::UserConfig;
use warp::http::HeaderValue;
use warp::hyper::body::Bytes;
use warp::hyper::Method;
use warp::Filter as WarpFilter;
use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response, Headers};
mod aws;
mod shared;
mod user;

#[derive(Clone, serde::Serialize)]
struct Payload {
    resource: String,
    arn: String,
    status: String,
    port: u16,
}
#[tauri::command]
async fn user_config(user_config: tauri::State<'_, UserConfigState>) -> Result<UserConfig, BError> {
    let user_config = user_config.0.lock().await;
    Ok(user_config.clone())
}

#[tauri::command]
async fn favorite_ecs(
    arn: &str,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    user_config.favorite_ecs(arn)
}

#[tauri::command]
async fn favorite_rds(
    arn: &str,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    user_config.favorite_rds(arn)
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
        Command::new("aws")
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
async fn home(
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    home_cache: tauri::State<'_, HomeCache>,
    databases_cache: tauri::State<'_, DatabasesCache>,
) -> Result<HomePage, BError> {
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };
    if let Some(active_profile) = active_profile {
        let mut home_cache = home_cache.0.lock().await;
        let user = user_config.0.lock().await;
        let ecs_client = aws::ecs_client(&active_profile).await;

        let mut databases_cache = databases_cache.0.lock().await;
        {
            if databases_cache.is_empty() {
                let databases = aws::databases(&aws::rds_client(&active_profile).await).await;
                for db in databases {
                    databases_cache.push(db);
                }
            }
        }

        let dbs_list: Vec<aws::DbInstance> = databases_cache
            .iter()
            .filter(|db| user.rds.contains(&db.arn))
            .cloned()
            .collect();
        home_cache.databases = dbs_list;
        let ecs_arns = &user.ecs;
        let cached_services = &mut home_cache.services;
        for ecs in ecs_arns.into_iter() {
            if !cached_services
                .into_iter()
                .any(|cached_ecs| cached_ecs.arn == *ecs)
            {
                cached_services.push(aws::service_details(&ecs_client, ecs).await);
            }
        }

        Ok(home_cache.clone())
    } else {
        Err(BError::new("home", "Login required"))
    }
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

#[tauri::command]
async fn discover() -> Result<(), BError> {
    todo!()
}

#[tauri::command]
async fn start_db_proxy(
    window: Window,
    db: aws::DbInstance,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
) -> Result<(), BError> {
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };
    let local_port = user_config.0.lock().await.get_db_port(&db.arn);

    if let Some(active_profile) = active_profile {
        let ec2_client = aws::ec2_client(&active_profile).await;
        let bastions = aws::bastions(&ec2_client).await;
        let bastion = bastions
            .into_iter()
            .find(|b| b.env == db.env)
            .expect("No bastion found");

        start_aws_ssm_proxy(
            db.arn,
            window,
            bastion.instance_id,
            active_profile,
            db.endpoint.address,
            db.endpoint.port,
            local_port,
        );
        Ok(())
    } else {
        Err(BError::new("start_db_proxy", "Failed to start"))
    }
}

#[tauri::command]
async fn open_dbeaver(
    db: aws::DbInstance,
    port: u16,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
) -> Result<(), BError> {
    fn db_beaver_con_parma(db_name: &str, host: &str, port: u16, secret: &aws::DbSecret) -> String {
        if secret.auto_rotated {
            format!(
                "driver=postgresql|id={}|name={}|openConsole=true|folder=wombat|url=jdbc:postgresql://{}:{}/{}?user={}&password={}",
                db_name, db_name, host,port, secret.dbname, secret.username, encode(&secret.password)
                )
        } else {
            format!(
                "driver=postgresql|id={}|name={}|openConsole=true|folder=wombat|savePassword=true|create=true|save=true|host={}|port={}|database={}|user={}|password={}",
                db_name, db_name, host,port, secret.dbname, secret.username, &secret.password
                )
        }
    }

    let dbeaver_path = &user_config
        .0
        .lock()
        .await
        .dbeaver_path
        .as_ref()
        .expect("DBeaver needs to be configured")
        .clone();
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };

    if let Some(active_profile) = active_profile {
        let db_secret = aws::db_secret(
            &aws::secretsmanager_client(&active_profile).await,
            &aws::ssm_client(&active_profile).await,
            &db.name,
            &db.env,
        )
        .await;
        if db_secret.is_err() {
            return Err(db_secret.err().unwrap());
        }
        let db_secret = db_secret.unwrap();

        Command::new(dbeaver_path)
            .args([
                "-con",
                &db_beaver_con_parma(
                    &db.arn.split(":").last().unwrap(),
                    "localhost",
                    port,
                    &db_secret,
                ),
            ])
            .output()
            .expect("failed to execute process");
        return Ok(());
    } else {
        Err(BError::new("open_dbeaver", "Failed to start"))
    }
}

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
    arn: String,
    window: Window,
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
        window
            .emit(
                "db-proxy",
                Payload {
                    arn: arn.clone(),
                    resource: "db".into(),
                    status: "START".into(),
                    port: local_port,
                },
            )
            .unwrap();
        Command::new("aws")
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
        window
            .emit(
                "db-proxy",
                Payload {
                    arn: arn.clone(),
                    resource: "db".into(),
                    status: "END".into(),
                    port: local_port,
                },
            )
            .unwrap();
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
        .manage(HomeCache(Arc::new(Mutex::new(HomePage {
            services: Vec::new(),
            databases: Vec::new(),
        }))))
        .invoke_handler(tauri::generate_handler![
            user_config,
            login,
            logout,
            clusters,
            services,
            databases,
            set_dbeaver_path,
            favorite_ecs,
            favorite_rds,
            start_db_proxy,
            open_dbeaver,
            home,
            discover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct HomeCache(Arc<Mutex<HomePage>>);

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HomePage {
    services: Vec<aws::ServiceDetails>,
    databases: Vec<aws::DbInstance>,
}
