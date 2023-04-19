// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aws::{ecs_client, Cluster, DbInstance, EcsService, Env, ServiceDetails};

use chrono::prelude::*;
use shared::BError;
use std::sync::Arc;
use std::time::Duration;
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
struct ProxyEventMessage {
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
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,

    home_cache: tauri::State<'_, HomeCache>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,
    database_cache: tauri::State<'_, DatabasesCache>,
    cluster_cache: tauri::State<'_, ClustersCache>,
    service_cache: tauri::State<'_, ServicesCache>,
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

    let database_cache = Arc::clone(&database_cache.0);
    let db_profile = profile.to_owned().clone();
    tokio::task::spawn(async move {
        let db_cache = &mut database_cache.lock().await;
        populate_db_cache(&db_profile, db_cache).await;
    });

    let cluster_cache = Arc::clone(&cluster_cache.0);
    let cache_profile = profile.to_owned().clone();
    let cluster_handle = tokio::task::spawn(async move {
        let cluster_cache = &mut cluster_cache.lock().await;
        populate_cluster_cache(&cache_profile, cluster_cache).await;
        cluster_cache.clone()
    });

    let home_page_ref = Arc::clone(&home_cache.0);
    let job_profile = profile.to_owned();
    task_tracker.0.lock().await.home_details_refresher = Some(tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut home_page = home_page_ref.lock().await;
            if Utc::now()
                .signed_duration_since(home_page.timestamp)
                .num_minutes()
                > 10
            {
                let arns_to_update = home_page
                    .services
                    .iter()
                    .map(|s| s.arn.to_owned())
                    .collect::<Vec<String>>();
                let handles: Vec<_> = arns_to_update
                    .into_iter()
                    .map(|service| {
                        let job_profile = job_profile.clone();
                        return tokio::task::spawn(async move {
                            let ecs_client = aws::ecs_client(&job_profile).await;
                            aws::service_details(&ecs_client, &service).await
                        });
                    })
                    .collect();
                let mut new_services = vec![];
                for handle in handles {
                    new_services.push(handle.await.unwrap())
                }

                home_page.services = new_services
            }

            window.emit("new-home-cache", home_page.clone()).unwrap();
        }
    }));

    let service_job_cache = Arc::clone(&service_cache.0);
    let service_job_profile = profile.to_owned();
    tokio::task::spawn(async move {
        let cluster_cache = cluster_handle.await.unwrap();
        let service_cache = &mut service_job_cache.lock().await;
        populate_services_cache(&service_job_profile, &cluster_cache, service_cache).await;
    });

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
    service_cache: tauri::State<'_, ServicesCache>,
    db_cache: tauri::State<'_, DatabasesCache>,
    home_cache: tauri::State<'_, HomeCache>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    let mut app_state = app_state.0.lock().await;
    app_state.active_profile = None;
    service_cache.0.lock().await.clear();
    db_cache.0.lock().await.clear();
    home_cache.0.lock().await.databases.clear();
    home_cache.0.lock().await.services.clear();
    if let Some(handler) = &task_tracker.0.lock().await.home_details_refresher {
        handler.abort()
    }
    task_tracker.0.lock().await.home_details_refresher = None;

    Ok(())
}

async fn populate_db_cache(active_profile: &str, databases_cache: &mut Vec<DbInstance>) {
    databases_cache.clear();
    let databases = aws::databases(&aws::rds_client(&active_profile).await).await;
    for db in databases {
        databases_cache.push(db);
    }
}

async fn populate_cluster_cache(active_profile: &str, cluster_cache: &mut Vec<Cluster>) {
    cluster_cache.clear();
    let clusters = aws::clusters(&aws::ecs_client(&active_profile).await).await;
    for cluster in clusters {
        cluster_cache.push(cluster);
    }
}

async fn populate_services_cache(
    active_profile: &str,
    clusters: &[Cluster],
    services_cache: &mut HashMap<Env, Vec<EcsService>>,
) {
    services_cache.clear();
    for cluster in clusters {
        let services = aws::services(&aws::ecs_client(&active_profile).await, cluster).await;
        services_cache.insert(cluster.env.clone(), services);
    }
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

        let databases_cache = databases_cache.0.lock().await;
        let dbs_list: Vec<aws::DbInstance> = databases_cache
            .iter()
            .filter(|db| user.rds.contains(&db.arn))
            .cloned()
            .collect();
        home_cache.databases = dbs_list;

        let ecs_arns = user.ecs.clone();
        let mut ecs_list: Vec<ServiceDetails> = home_cache
            .services
            .iter()
            .filter(|e| ecs_arns.contains(&e.arn))
            .cloned()
            .collect();

        let cached_services = &mut home_cache.services;
        let mut handles: Vec<_> = vec![];
        for ecs in ecs_arns.into_iter() {
            if !cached_services
                .into_iter()
                .any(|cached_ecs| cached_ecs.arn == *ecs)
            {
                let ecs_client = aws::ecs_client(&active_profile.clone()).await;
                handles.push(tokio::task::spawn(async move {
                    aws::service_details(&ecs_client, &ecs).await
                }));
            }
        }
        for handle in handles {
            ecs_list.push(handle.await.unwrap())
        }

        home_cache.services = ecs_list;

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
async fn discover(
    name: &str,
    db_cache: tauri::State<'_, DatabasesCache>,
    service_cache: tauri::State<'_, ServicesCache>,
) -> Result<Vec<(String, aws::Env, String, String)>, BError> {
    let name = &name.to_lowercase();
    let mut records: Vec<(String, aws::Env, String, String)> = vec![];
    db_cache
        .0
        .lock()
        .await
        .iter()
        .filter(|db| db.arn.contains(name))
        .map(|db| {
            (
                "Database".to_owned(),
                db.env.clone(),
                db.arn.to_owned(),
                db.name.to_owned(),
            )
        })
        .for_each(|r| records.push(r));
    for entry in service_cache.0.lock().await.iter() {
        entry
            .1
            .iter()
            .filter(|s| s.arn.contains(name))
            .map(|s| {
                (
                    "Service".to_owned(),
                    s.env.clone(),
                    s.arn.to_owned(),
                    s.name.to_owned(),
                )
            })
            .for_each(|r| records.push(r));
    }

    Ok(records)
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
            None,
            local_port,
        );

        Ok(())
    } else {
        Err(BError::new("start_db_proxy", "Failed to start"))
    }
}

#[tauri::command]
async fn refresh_cache(
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    databse_cache: tauri::State<'_, DatabasesCache>,
    cluster_cache: tauri::State<'_, ClustersCache>,
    service_cache: tauri::State<'_, ServicesCache>,
    home_cache: tauri::State<'_, HomeCache>,
) -> Result<(), ()> {
    let profile = &app_state.0.lock().await.active_profile.clone().unwrap();

    let db_cache = &mut databse_cache.0.lock().await;
    populate_db_cache(profile, db_cache).await;

    let cluster_cache = &mut cluster_cache.0.lock().await;
    populate_cluster_cache(profile, cluster_cache).await;

    let service_cache = &mut service_cache.0.lock().await;
    populate_services_cache(profile, &cluster_cache, service_cache).await;

    home_cache.0.lock().await.services.clear();
    home_cache.0.lock().await.databases.clear();
    window.emit("cache-refreshed", ()).unwrap();
    Ok(())
}

#[tauri::command]
async fn start_service_proxy(
    window: Window,
    service: aws::EcsService,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
) -> Result<(), BError> {
    let active_profile = {
        let app_state = app_state.0.lock().await;
        app_state.active_profile.clone()
    };
    let local_port = user_config.0.lock().await.get_service_port(&service.arn);
    let aws_local_port = local_port + 10000;

    if let Some(active_profile) = active_profile {
        let ec2_client = aws::ec2_client(&active_profile).await;
        let bastions = aws::bastions(&ec2_client).await;
        let bastion = bastions
            .into_iter()
            .find(|b| b.env == service.env)
            .expect("No bastion found");
        let host = format!("{}.service", service.name);
        let handle = start_proxy_to_aws_proxy(Some(host.clone()), local_port, aws_local_port).await;
        start_aws_ssm_proxy(
            service.arn,
            window,
            bastion.instance_id,
            active_profile,
            host,
            80,
            aws_local_port,
            Some(handle),
            local_port,
        );

        Ok(())
    } else {
        Err(BError::new("start_service_proxy", "Failed to start"))
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

async fn start_proxy_to_aws_proxy(
    service_header: Option<String>,
    local_port: u16,
    aws_local_port: u16,
) -> tokio::task::JoinHandle<()> {
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
                headers.insert("Origin", header_value.clone());
                headers.insert("Host", header_value.clone());

                proxy_to_and_forward_response(
                    format!("http://localhost:{}/", aws_local_port).to_owned(),
                    "".to_owned(),
                    uri,
                    params,
                    method,
                    headers,
                    body,
                )
            },
        );
        warp::serve(app).run(([0, 0, 0, 0], local_port)).await;
    })
}

fn start_aws_ssm_proxy(
    arn: String,
    window: Window,
    bastion: String,
    profile: String,
    target: String,
    target_port: u16,
    local_port: u16,

    abort_on_exit: Option<tokio::task::JoinHandle<()>>,
    access_port: u16,
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
                "proxy-start",
                ProxyEventMessage {
                    arn: arn.clone(),
                    status: "START".into(),
                    port: access_port,
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

        if let Some(handle) = abort_on_exit {
            handle.abort()
        }
        window
            .emit(
                "proxy-end",
                ProxyEventMessage {
                    arn: arn.clone(),
                    status: "END".into(),
                    port: access_port,
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
        .manage(DatabasesCache::default())
        .manage(ServicesCache::default())
        .manage(ClustersCache::default())
        .manage(AsyncTaskManager(Arc::new(Mutex::new(TaskTracker {
            home_details_refresher: None,
        }))))
        .manage(HomeCache(Arc::new(Mutex::new(HomePage {
            timestamp: Utc::now(),
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
            start_service_proxy,
            open_dbeaver,
            home,
            discover,
            refresh_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct HomeCache(Arc<Mutex<HomePage>>);

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
    timestamp: DateTime<Utc>,
    services: Vec<aws::ServiceDetails>,
    databases: Vec<aws::DbInstance>,
}
struct AsyncTaskManager(Arc<Mutex<TaskTracker>>);

struct TaskTracker {
    home_details_refresher: Option<tokio::task::JoinHandle<()>>,
}
