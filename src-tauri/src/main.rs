// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use aws::{cloudwatchlogs_client, Cluster, DbInstance, DbSecret, EcsService, ServiceDetails};
use axiom_rs::Client;
use log::{error, info, warn, LevelFilter};
use regex::Regex;
use serde_json::json;
use shared::{ecs_arn_to_name, rds_arn_to_name};
use shared::{BError, Env};
use shared_child::SharedChild;
use std::collections::HashSet;
use std::env;
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, process::Command};
use tauri::Window;
use tauri_plugin_log::LogTarget;
use tokio::sync::Mutex;
use tracing_unwrap::{OptionExt, ResultExt};
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

#[derive(Clone, serde::Serialize)]
struct ServiceDetailsPayload {
    app: String,
    services: Vec<aws::ServiceDetails>,
    dbs: Vec<aws::DbInstance>,
}

#[tauri::command]
async fn user_config(user_config: tauri::State<'_, UserConfigState>) -> Result<UserConfig, BError> {
    let user_config = user_config.0.lock().await;
    Ok(user_config.clone())
}

#[tauri::command]
async fn favorite(
    name: &str,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;

    ingest_log(
        &axiom.0,
        &user_config.id,
        Action::UpdateTrackedNames(name.to_owned()),
        None,
        None,
    )
    .await;

    user_config.favorite(name.to_owned())
}

#[tauri::command]
async fn login(
    profile: &str,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,
    rds_state: tauri::State<'_, RdsClientState>,
    ecs_state: tauri::State<'_, EcsClientState>,
) -> Result<UserConfig, BError> {
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    ingest_log(
        &axiom.0,
        &user_config.0.lock().await.id,
        Action::Login(profile.to_owned()),
        None,
        None,
    )
    .await;

    let mut user_config = user_config.0.lock().await;
    user_config.use_profile(profile);
    app_state.0.lock().await.active_profile = Some(profile.to_owned());
    let user_id = user_config.id.clone();

    {
        let rds_client = &mut rds_state.0.lock().await;
        rds_client.init(profile).await;
        let dbs = rds_client.databases().await;
        ingest_log(
            &axiom.0,
            &user_id,
            Action::RefreshRdsList,
            None,
            Some(dbs.len()),
        )
        .await;
    }

    {
        let mut ecs_client = ecs_state.0.lock().await;
        ecs_client.init(profile).await;
    }
    {
        let clusters = ecs_state.0.lock().await.clusters().await;
        ingest_log(
            &axiom.0,
            &user_id,
            Action::RefreshClusterList,
            None,
            Some(clusters.len()),
        )
        .await;
        for cluster in clusters {
            let services = ecs_state.0.lock().await.services(&cluster).await;
            ingest_log(
                &axiom.0,
                &user_id,
                Action::RefreshEcsList(cluster.name.to_owned(), cluster.env.clone()),
                None,
                Some(services.len()),
            )
            .await;
        }
    }
    let ecs_arc_clone = Arc::clone(&ecs_state.0);
    let rds_arc_clone = Arc::clone(&rds_state.0);
    let refresher_axiom = Arc::clone(&axiom.0);
    let refresher_user_id = user_config.id.clone();
    task_tracker.0.lock().await.aws_resource_refresher = Some(tokio::task::spawn(async move {
        let initial_wait = tokio::time::sleep(Duration::from_secs(30 * 60));
        initial_wait.await;
        let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
        loop {
            interval.tick().await;
            {
                let rds = &mut rds_arc_clone.lock().await;
                rds.clear();
                let dbs = rds.databases().await;
                if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                    ingest_log_with_client(
                        axiom,
                        &refresher_user_id,
                        Action::RefreshRdsList,
                        None,
                        Some(dbs.len()),
                    )
                    .await;
                }
            }
            {
                let ecs = &mut ecs_arc_clone.lock().await;
                ecs.clear_shallow();
                let clusters = ecs.clusters().await;
                if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                    ingest_log_with_client(
                        axiom,
                        &refresher_user_id,
                        Action::RefreshClusterList,
                        None,
                        Some(clusters.len()),
                    )
                    .await;
                }

                for cluster in clusters {
                    let services = ecs.services(&cluster).await;
                    if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                        ingest_log_with_client(
                            axiom,
                            &refresher_user_id,
                            Action::RefreshEcsList(cluster.name, cluster.env),
                            None,
                            Some(services.len()),
                        )
                        .await;
                    }
                }
            }
        }
    }));

    Ok(user_config.clone())
}

#[tauri::command]
async fn set_dbeaver_path(
    dbeaver_path: &str,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    ingest_log(
        &axiom.0,
        &user_config.id,
        Action::SetDbeaverPath(dbeaver_path.to_owned()),
        None,
        None,
    )
    .await;
    user_config.set_dbeaver_path(dbeaver_path)
}

#[tauri::command]
async fn save_preffered_envs(
    envs: Vec<shared::Env>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    ingest_log(
        &axiom.0,
        &user_config.id,
        Action::SetPrefferedEnvs(envs.clone()),
        None,
        None,
    )
    .await;
    user_config.save_preffered_envs(envs)
}

#[tauri::command]
async fn credentials(
    db: aws::DbInstance,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    db_state: tauri::State<'_, RdsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<DbSecret, BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let user_id = &user_config.0.lock().await.id.clone();
    let login_check = check_login_and_trigger(user_id, &profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }
    let secret = db_state.0.lock().await.db_secret(&db.name, &db.env).await;
    match &secret {
        Ok(_) => {
            ingest_log(
                &axiom.0,
                user_id,
                Action::FetchCredentials(db.name, db.env),
                None,
                None,
            )
            .await
        }
        Err(err) => {
            ingest_log(
                &axiom.0,
                user_id,
                Action::FetchCredentials(db.name, db.env),
                Some(err.message.clone()),
                None,
            )
            .await
        }
    };
    return secret;
}

#[tauri::command]
async fn stop_job(
    arn: &str,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,

    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let user_id = &user_config.0.lock().await.id.clone();
    if let Some(job) = async_task_tracker
        .0
        .lock()
        .await
        .proxies_handlers
        .remove(arn)
    {
        ingest_log(
            &axiom.0,
            &user_id,
            Action::StopJob(
                shared::arn_to_name(arn).to_owned(),
                Env::from_any(arn).to_owned(),
                shared::arn_resource_type(arn).to_owned(),
            ),
            None,
            None,
        )
        .await;
        let _ = job.kill();
        let _ = job.wait();
        let mut out = job.take_stdout();
        let mut session_id: Option<String> = None;
        let session_regex = Regex::new("Starting session with SessionId: (.*)").unwrap_or_log();

        if let Some(stdout) = &mut out {
            let lines = BufReader::new(stdout).lines().enumerate().take(10);
            for (_counter, line) in lines {
                if let Ok(line) = line {
                    let captures = session_regex.captures(&line);
                    let found_session_id = captures
                        .and_then(|c| c.get(1))
                        .and_then(|e| Some(e.as_str().to_owned()));
                    if found_session_id.is_some() {
                        session_id = found_session_id;
                        break;
                    }
                }
            }
        }
        if let Some(session_id) = session_id {
            let app_ctx = app_state.0.lock().await;
            let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
            let killed_session_output = Command::new("aws")
                .args([
                    "ssm",
                    "terminate-session",
                    "--session-id",
                    &session_id,
                    "--profile",
                    &profile,
                ])
                .output();
            match killed_session_output {
                Ok(output) => info!("Attempted to kill session in SSM: {:?}", output),
                Err(e) => warn!("Failed to kill session in SSM {}", e),
            };
        } else {
            info!(
                "SessionId {} to kill not found",
                session_id.unwrap_or_default()
            )
        }
    } else {
        ingest_log(
            &axiom.0,
            &user_id,
            Action::StopJob(
                shared::arn_to_name(arn).to_owned(),
                Env::from_any(arn).to_owned(),
                shared::arn_resource_type(arn).to_owned(),
            ),
            Some(String::from("No matching job running!")),
            None,
        )
        .await;
    }

    Ok(())
}

#[tauri::command]
async fn logout(
    app_state: tauri::State<'_, AppContextState>,
    ecs_state: tauri::State<'_, EcsClientState>,
    rds_state: tauri::State<'_, RdsClientState>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_state: tauri::State<'_, UserConfigState>,
) -> Result<(), BError> {
    let mut app_state = app_state.0.lock().await;
    if let Some(profile) = app_state.active_profile.as_ref() {
        ingest_log(
            &axiom.0,
            &user_state.0.lock().await.id,
            Action::Logout(profile.clone()),
            None,
            None,
        )
        .await;
    }

    let home_details_refresher = &mut task_tracker.0.lock().await;
    if let Some(handler) = &home_details_refresher.aws_resource_refresher {
        handler.abort()
    }
    let jobs = &mut home_details_refresher.proxies_handlers;
    for job in jobs.drain() {
        let _ = job.1.kill();
        let _ = job.1.wait();
    }
    {
        ecs_state.0.lock().await.shutdown();
        rds_state.0.lock().await.shutdown();
    }
    {
        app_state.active_profile = None;
    }
    Ok(())
}

struct WindowNotifier {
    window: Window,
}

impl aws::OnLogFound for WindowNotifier {
    fn notify(&self, log: aws::LogEntry) {
        let _ = self.window.emit("new-log-found", log);
    }
    fn success(&self) {
        let _ = self.window.emit("find-logs-success", ());
    }
    fn error(&self, msg: String) {
        let _ = self.window.emit("find-logs-error", msg);
    }
}

#[tauri::command]
async fn find_logs(
    app: String,
    env: Env,
    start: i64,
    end: i64,
    filter: String,
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_config: tauri::State<'_, UserConfigState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log().to_owned();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, &profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }
    {
        let handler = &async_task_tracker.0.lock().await.search_log_handler;
        if let Some(handler) = handler {
            handler.abort()
        }
    }
    {
        async_task_tracker.0.lock().await.search_log_handler = None;
    }
    let user_id = user_config.0.lock().await.id.clone();

    ingest_log(
        &axiom.0,
        &user_id,
        Action::StartSearchLogs(app.to_owned(), env.clone(), filter.to_string(), end - start),
        None,
        None,
    )
    .await;

    let axiom = Arc::clone(&axiom.0);
    async_task_tracker.0.lock().await.search_log_handler = Some(tokio::task::spawn(async move {
        let notifier = Arc::new(Mutex::new(WindowNotifier { window }));
        let action =
            Action::SearchLogs(app.to_owned(), env.clone(), filter.to_string(), end - start);
        let result = aws::find_logs(
            cloudwatchlogs_client(profile.as_str()).await,
            env,
            app,
            start,
            end,
            filter,
            notifier.clone(),
        )
        .await;
        // let axiom = axiom.lock().await;
        match &result {
            Ok(cnt) => ingest_log(&axiom, &user_id, action, None, Some(*cnt)).await,
            Err(err) => {
                ingest_log(&axiom, &user_id, action, Some(err.message.to_owned()), None).await
            }
        }
    }));

    return Ok(());
}

#[tauri::command]
async fn abort_find_logs(
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    info!("Aborting find logs");
    let mut tracker = async_task_tracker.0.lock().await;
    if let Some(handler) = &tracker.search_log_handler {
        handler.abort()
    }
    tracker.search_log_handler = None;

    Ok(())
}

#[tauri::command]
async fn clusters(
    app_state: tauri::State<'_, AppContextState>,
    ecs_state: tauri::State<'_, EcsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<Vec<aws::Cluster>, BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    Ok(ecs_state.0.lock().await.clusters().await)
}

#[tauri::command]
async fn services(
    cluster: Cluster,
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, EcsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<Vec<aws::EcsService>, BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    Ok(cache.0.lock().await.services(&cluster).await)
}

#[tauri::command]
async fn databases(
    env: shared::Env,
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, RdsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<Vec<aws::DbInstance>, BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    let dbs = cache.0.lock().await.databases().await;

    Ok(dbs.into_iter().filter(|db| db.env == env).collect())
}

#[tauri::command]
async fn discover(
    name: &str,
    app_state: tauri::State<'_, AppContextState>,
    db_cache: tauri::State<'_, RdsClientState>,
    user_config: tauri::State<'_, UserConfigState>,
    service_cache: tauri::State<'_, EcsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_state: tauri::State<'_, UserConfigState>,
) -> Result<Vec<String>, BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    let name = &name.to_lowercase();
    let tracked_names = user_config.0.lock().await.tracked_names.clone();
    let mut found_names = HashSet::new();
    if name.len() < 1 {
        ingest_log(
            &axiom.0,
            &user_state.0.lock().await.id,
            Action::Discover(name.to_owned()),
            None,
            Some(0),
        )
        .await;
        return Ok(Vec::new());
    }
    let mut dbs: HashMap<String, Vec<aws::DbInstance>> = HashMap::new();
    {
        let rds_client = &mut db_cache.0.lock().await;
        let found_dbs: Vec<DbInstance> = rds_client
            .databases()
            .await
            .into_iter()
            .filter(|db| {
                db.arn.contains(name) && !tracked_names.contains(&rds_arn_to_name(&db.arn))
            })
            .collect();

        for db in found_dbs {
            let name = rds_arn_to_name(&db.arn);
            if !dbs.contains_key(&name) {
                dbs.insert(name, vec![db]);
            } else {
                dbs.get_mut(&name).unwrap_or_log().push(db);
            }
        }
    }

    let mut services: HashMap<String, Vec<ServiceDetails>> = HashMap::new();
    {
        let ecs_client = &mut service_cache.0.lock().await;
        let clusters = ecs_client.clusters().await;
        for cluster in clusters.iter() {
            let matching_services_at_cluster: Vec<EcsService> = ecs_client
                .services(cluster)
                .await
                .into_iter()
                .filter(|s| {
                    s.arn.contains(name) && !tracked_names.contains(&ecs_arn_to_name(&s.arn))
                })
                .collect();
            for service in matching_services_at_cluster {
                let name = ecs_arn_to_name(&service.arn);
                let sd = ecs_client.service_details(&service.arn, false).await;
                if !services.contains_key(&name) {
                    services.insert(name, vec![sd]);
                } else {
                    services.get_mut(&name).unwrap().push(sd);
                }
            }
        }
    }

    found_names.extend(dbs.keys().cloned());
    found_names.extend(services.keys().cloned());

    ingest_log(
        &axiom.0,
        &user_state.0.lock().await.id,
        Action::Discover(name.to_owned()),
        None,
        Some(found_names.len()),
    )
    .await;

    Ok(found_names.into_iter().collect())
}

#[tauri::command]
async fn start_db_proxy(
    window: Window,
    db: aws::DbInstance,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    let local_port = user_config.0.lock().await.get_db_port(&db.arn);

    let ec2_client = aws::ec2_client(profile).await;
    let bastions = aws::bastions(&ec2_client).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == db.env)
        .expect("No bastion found");

    ingest_log(
        &axiom.0,
        &user_config.0.lock().await.id,
        Action::StartRdsProxy(db.name.to_owned(), db.env.clone()),
        None,
        None,
    )
    .await;

    start_aws_ssm_proxy(
        db.arn,
        window,
        bastion.instance_id,
        profile.to_owned(),
        db.endpoint.address,
        db.endpoint.port,
        local_port,
        None,
        local_port,
        async_task_tracker,
    )
    .await;

    Ok(())
}

#[tauri::command]
async fn refresh_cache(
    app_state: tauri::State<'_, AppContextState>,
    window: Window,
    databse_cache: tauri::State<'_, RdsClientState>,
    service_cache: tauri::State<'_, EcsClientState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    ingest_log(
        &axiom.0,
        &user_config.0.lock().await.id,
        Action::ClearCache,
        None,
        None,
    )
    .await;
    let db_cache = &mut databse_cache.0.lock().await;
    db_cache.clear();
    db_cache.databases().await;

    let service_cache = &mut service_cache.0.lock().await;
    service_cache.clear();
    let clusters = service_cache.clusters().await;
    for cluster in clusters {
        service_cache.services(&cluster).await;
    }

    window.emit("cache-refreshed", ()).unwrap_or_log();
    Ok(())
}

#[tauri::command]
async fn service_details(
    app: String,
    window: Window,
    user_config: tauri::State<'_, UserConfigState>,
    databases_cache: tauri::State<'_, RdsClientState>,
    services_cache: tauri::State<'_, EcsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let db = Arc::clone(&databases_cache.0);
    let ecs = Arc::clone(&services_cache.0);
    ingest_log(
        &axiom.0,
        &user_config.0.lock().await.id,
        Action::ServiceDetails(app.to_owned()),
        None,
        None,
    )
    .await;
    info!("Called service_details: {}", &app);

    tokio::task::spawn(async move {
        info!("Fetching details for: {}", &app);
        let mut dbs_list: Vec<aws::DbInstance> = vec![];
        {
            let databases_cache = &mut db.lock().await;
            let databases = databases_cache.databases().await;
            for db in databases.into_iter() {
                if &shared::rds_arn_to_name(&db.arn) == &app {
                    dbs_list.push(db.clone())
                }
            }
        }

        let mut services: Vec<ServiceDetails> = vec![];
        {
            let services_map = ecs
                .lock()
                .await
                .service_details_for_names(&HashSet::from([app.clone()]), false)
                .await;
            for service_list in services_map.into_values() {
                for service in service_list {
                    services.push(service)
                }
            }
        }

        window
            .emit(
                "new-service-details",
                ServiceDetailsPayload {
                    app: app,
                    services: services,
                    dbs: dbs_list,
                },
            )
            .unwrap_or_log();
    });
    Ok(())
}

#[tauri::command]
async fn start_service_proxy(
    window: Window,
    service: aws::EcsService,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let local_port = user_config.0.lock().await.get_service_port(&service.arn);
    let aws_local_port = local_port + 10000;

    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    let ec2_client = aws::ec2_client(profile).await;
    let bastions = aws::bastions(&ec2_client).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == service.env)
        .expect("No bastion found");
    let host = format!("{}.service", service.name);
    ingest_log(
        &axiom.0,
        &user_config.0.lock().await.id,
        Action::StartEcsProxy(service.name.to_owned(), service.env.clone()),
        None,
        None,
    )
    .await;

    let handle = start_proxy_to_aws_proxy(Some(host.clone()), local_port, aws_local_port).await;

    start_aws_ssm_proxy(
        service.arn,
        window,
        bastion.instance_id,
        profile.to_owned(),
        host,
        80,
        aws_local_port,
        Some(handle),
        local_port,
        async_task_tracker,
    )
    .await;

    Ok(())
}

#[tauri::command]
async fn open_dbeaver(
    db: aws::DbInstance,
    port: u16,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    rds_state: tauri::State<'_, RdsClientState>,
    axiom: tauri::State<'_, AxiomClientState>,
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
    let app_ctx = app_state.0.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
    let login_check =
        check_login_and_trigger(&user_config.0.lock().await.id, profile, &axiom).await;
    if login_check.is_err() {
        return Err(login_check.expect_err("It supposed to be err"));
    }

    let db_secret = rds_state.0.lock().await.db_secret(&db.name, &db.env).await;
    if let Err(err) = db_secret {
        ingest_log(
            &axiom.0,
            &user_config.0.lock().await.id,
            Action::OpenDbeaver(db.name.to_owned(), db.env.clone()),
            Some(err.message.clone()),
            None,
        )
        .await;
        return Err(err);
    }
    let db_secret = db_secret.unwrap_or_log();
    ingest_log(
        &axiom.0,
        &user_config.0.lock().await.id,
        Action::OpenDbeaver(db.name.to_owned(), db.env.clone()),
        None,
        None,
    )
    .await;
    Command::new(dbeaver_path)
        .args([
            "-con",
            &db_beaver_con_parma(
                &db.arn.split(":").last().unwrap_or_log(),
                "localhost",
                port,
                &db_secret,
            ),
        ])
        .output()
        .expect("failed to execute process");
    return Ok(());
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
            .unwrap_or_log();
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

async fn start_aws_ssm_proxy(
    arn: String,
    window: Window,
    bastion: String,
    profile: String,
    target: String,
    target_port: u16,
    local_port: u16,

    abort_on_exit: Option<tokio::task::JoinHandle<()>>,
    access_port: u16,
    async_task_manager: tauri::State<'_, AsyncTaskManager>,
) {
    let mut command = Command::new("aws");
    command.args([
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
    ]);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    info!("Execudtnig cmd: {:?} ", command);
    let shared_child = SharedChild::spawn(&mut command).unwrap_or_log();
    let shared_child_arc = Arc::new(shared_child);
    let child_arc_clone = shared_child_arc.clone();

    async_task_manager
        .0
        .lock()
        .await
        .proxies_handlers
        .insert(arn.clone(), shared_child_arc);

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
            .unwrap_or_log();
        let _ = child_arc_clone.wait();

        if let Some(handle) = abort_on_exit {
            info!("Killing dependant job");
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
            .unwrap_or_log();
    });
}

#[tokio::main]
async fn main() {
    fix_path_env::fix().unwrap_or_log();

    let user = UserConfig::default();

    let client = Client::builder()
        .with_token("%%AXIOM_TOKEN%%")
        .with_org_id("%%AXIOM_ORG%%")
        .build();
    let axiom_client = match format!("%%{}%%", "AXIOM_TOKEN") == "%%AXIOM_TOKEN%%" {
        true => AxiomClientState(Arc::new(Mutex::new(None))),
        false => AxiomClientState(Arc::new(Mutex::new(match client {
            Ok(client) => {
                ingest_log_with_client(&client, &user.id, Action::Start, None, None).await;
                Some(client)
            }
            Err(_) => None,
        }))),
    };
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([LogTarget::LogDir, LogTarget::Stdout, LogTarget::Webview])
                .level(LevelFilter::Info)
                .build(),
        )
        .manage(UserConfigState(Arc::new(Mutex::new(user))))
        .manage(axiom_client)
        .manage(AppContextState::default())
        .manage(RdsClientState(Arc::new(Mutex::new(aws::RdsClient::new()))))
        .manage(EcsClientState(Arc::new(Mutex::new(aws::EcsClient::new()))))
        .manage(AsyncTaskManager(Arc::new(Mutex::new(TaskTracker {
            aws_resource_refresher: None,
            proxies_handlers: HashMap::new(),
            search_log_handler: None,
        }))))
        .invoke_handler(tauri::generate_handler![
            user_config,
            set_dbeaver_path,
            save_preffered_envs,
            login,
            logout,
            clusters,
            services,
            databases,
            service_details,
            favorite,
            start_db_proxy,
            start_service_proxy,
            open_dbeaver,
            discover,
            refresh_cache,
            credentials,
            stop_job,
            find_logs,
            abort_find_logs
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}

struct RdsClientState(Arc<Mutex<aws::RdsClient>>);

struct EcsClientState(Arc<Mutex<aws::EcsClient>>);

#[derive(Default)]
struct AppContext {
    active_profile: Option<String>,
}

#[derive(Default)]
struct AppContextState(Arc<Mutex<AppContext>>);

struct AxiomClientState(Arc<Mutex<Option<axiom_rs::Client>>>);

struct UserConfigState(Arc<Mutex<UserConfig>>);

struct AsyncTaskManager(Arc<Mutex<TaskTracker>>);

struct TaskTracker {
    aws_resource_refresher: Option<tokio::task::JoinHandle<()>>,
    proxies_handlers: HashMap<String, Arc<SharedChild>>,
    search_log_handler: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HomeEntry {
    tracked_name: shared::TrackedName,
    services: HashMap<String, aws::ServiceDetails>,
    dbs: Vec<aws::DbInstance>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ActionLog {
    user_id: uuid::Uuid,
    action: Action,
    app_version: String,
    profile: String,
    error_message: Option<String>,
    record_count: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum Action {
    Start,
    Login(String),
    FetchCredentials(String, Env),
    StopJob(String, Env, String),
    StartEcsProxy(String, Env),
    ServiceDetails(String),
    StartRdsProxy(String, Env),
    OpenDbeaver(String, Env),
    LoginCheck(String),
    RefreshServiceDetails,
    RefreshRdsList,
    RefreshEcsList(String, Env),
    RefreshClusterList,
    Message(String),
    ClearCache,
    UpdateTrackedNames(String),
    SetDbeaverPath(String),
    SetPrefferedEnvs(Vec<Env>),
    Discover(String),
    Logout(String),
    StartSearchLogs(String, Env, String, i64),
    SearchLogs(String, Env, String, i64),
}

async fn check_login_and_trigger(
    user_id: &uuid::Uuid,
    profile: &str,
    axiom: &tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let client = aws::ecs_client(profile).await;
    if !aws::is_logged(&client).await {
        info!("Trigger log in into AWS");
        Command::new("aws")
            .args(["sso", "login", "--profile", &profile])
            .output()
            .expect("failed to execute process");
        if !aws::is_logged(&client).await {
            ingest_log(
                &axiom.0,
                user_id,
                Action::LoginCheck(profile.to_owned()),
                Some(String::from("Failed to log in.")),
                None,
            )
            .await;
            return Err(BError::new("login", "Failed to log in"));
        } else {
            ingest_log(
                &axiom.0,
                user_id,
                Action::LoginCheck(profile.to_owned()),
                None,
                None,
            )
            .await;
            return Ok(());
        }
    }
    Ok(())
}

async fn ingest_log(
    client: &Arc<Mutex<Option<Client>>>,
    user_id: &uuid::Uuid,
    action: Action,
    error_message: Option<String>,
    record_count: Option<usize>,
) {
    if let Some(client) = client.lock().await.as_ref() {
        ingest_log_with_client(&client, user_id, action, error_message, record_count).await;
    }
}

async fn ingest_log_with_client(
    client: &Client,
    user_id: &uuid::Uuid,
    action: Action,
    error_message: Option<String>,
    record_count: Option<usize>,
) {
    if let Err(e) = client
        .ingest(
            "wombat",
            vec![json!(ActionLog {
                user_id: user_id.clone(),
                action,
                error_message,
                record_count,
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                profile: String::from("%%PROFILE%%")
            })],
        )
        .await
    {
        error!("Error ingesting logs {}", e)
    }
}
