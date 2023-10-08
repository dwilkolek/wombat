// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use aws::{Cluster, DbInstance, DbSecret, EcsService, ServiceDetails};
use axiom_rs::Client;
use log::{error, info, warn, LevelFilter};
use regex::Regex;
use serde_json::json;
use shared::{arn_resource_type, arn_to_name, ecs_arn_to_name, rds_arn_to_name, ResourceType};
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
    name: String,
    env: Env,
    proxy_type: ResourceType,
}
impl ProxyEventMessage {
    fn new(arn: String, status: String, port: u16) -> Self {
        Self {
            arn: arn.clone(),
            status,
            port,
            name: arn_to_name(&arn),
            env: Env::from_any(&arn),
            proxy_type: arn_resource_type(&arn).unwrap_or_log(),
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ServiceDetailsPayload {
    app: String,
    services: Vec<aws::ServiceDetails>,
    dbs: Vec<aws::DbInstance>,
}

#[derive(Clone)]
struct AuthorizedUser {
    id: uuid::Uuid,
    profile: String,
    sdk_config: aws_config::SdkConfig,
}

const CHECK_AUTH_AFTER: i64 = 600_000;

async fn get_authorized(
    app_state: &Arc<Mutex<AppContext>>,
    axiom: &Arc<Mutex<Option<axiom_rs::Client>>>,
) -> AuthorizedUser {
    let mut app_ctx = app_state.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log().clone();
    let user_id = app_ctx.user_id.clone();
    let last_check = app_ctx.last_auth_check;
    

    let now = chrono::Local::now().timestamp_millis();
    if now - last_check > CHECK_AUTH_AFTER {
        let config = app_ctx.sdk_config.as_ref().unwrap_or_log().clone();
        {
            let login_check = check_login_and_trigger(&user_id, &profile, &config, &axiom).await;
            if login_check.is_err() {
                panic!("Authentication failed")
            }
        }
        app_ctx.last_auth_check = now;
    }
    return AuthorizedUser {
        profile: profile.to_owned(),
        id: app_ctx.user_id.clone(),
        sdk_config: app_ctx
            .sdk_config
            .as_ref()
            .expect("Sdk Config should be initialized at all times")
            .clone(),
    };
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
    database_cache: tauri::State<'_, DatabaseCache>,
    cluster_cache: tauri::State<'_, ClusterCache>,
    service_cache: tauri::State<'_, ServiceCache>,
    service_details_cache: tauri::State<'_, ServiceDetailsCache>,
) -> Result<UserConfig, BError> {
    {
        let mut app_state = app_state.0.lock().await;
        app_state.active_profile = Some(profile.to_owned());
        app_state.sdk_config = Some(aws_config::from_env().profile_name(profile).load().await);
    }
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::Login(profile.to_owned()),
        None,
        None,
    )
    .await;

    let mut user_config = user_config.0.lock().await;
    user_config.use_profile(profile);

    {
        let dbs = aws::databases(&authorized_user.sdk_config, Arc::clone(&database_cache.0)).await;
        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::RefreshRdsList,
            None,
            Some(dbs.len()),
        )
        .await;
    }

    let clusters;
    {
        clusters = aws::clusters(&authorized_user.sdk_config, Arc::clone(&cluster_cache.0)).await;
        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::RefreshClusterList,
            None,
            Some(clusters.len()),
        )
        .await;
    }
    {
        let cluster_services = aws::services(
            &authorized_user.sdk_config,
            clusters,
            Arc::clone(&service_cache.0),
        )
        .await;
        for (cluster, services) in cluster_services {
            ingest_log(
                &axiom.0,
                &authorized_user.id,
                Action::RefreshEcsList(cluster.name, cluster.env),
                None,
                Some(services.len()),
            )
            .await;
        }
    }

    let refresher_axiom = Arc::clone(&axiom.0);
    let refresher_user_id = authorized_user.id.clone();
    let authorized_user = authorized_user.clone();

    let database_cache = Arc::clone(&database_cache.0);
    let cluster_cache = Arc::clone(&cluster_cache.0);
    let service_cache = Arc::clone(&service_cache.0);
    let service_details_cache = Arc::clone(&service_details_cache.0);

    task_tracker.0.lock().await.aws_resource_refresher = Some(tokio::task::spawn(async move {
        let initial_wait = tokio::time::sleep(Duration::from_secs(30 * 60));
        initial_wait.await;
        let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
        loop {
            interval.tick().await;
            {
                {
                    database_cache.lock().await.clear();
                }
                let dbs =
                    aws::databases(&authorized_user.sdk_config, Arc::clone(&database_cache)).await;
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

            let clusters;
            {
                {
                    cluster_cache.lock().await.clear();
                }
                clusters =
                    aws::clusters(&authorized_user.sdk_config, Arc::clone(&cluster_cache)).await;
                if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                    ingest_log_with_client(
                        axiom,
                        &authorized_user.id,
                        Action::RefreshClusterList,
                        None,
                        Some(clusters.len()),
                    )
                    .await;
                }
            }
            {
                {
                    service_cache.lock().await.clear();
                }
                let cluster_services = aws::services(
                    &authorized_user.sdk_config,
                    clusters,
                    Arc::clone(&service_cache),
                )
                .await;

                for (cluster, services) in cluster_services {
                    if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                        ingest_log_with_client(
                            axiom,
                            &authorized_user.id,
                            Action::RefreshEcsList(cluster.name, cluster.env),
                            None,
                            Some(services.len()),
                        )
                        .await;
                    }
                }
            }
            {
                service_details_cache.lock().await.clear();
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
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<DbSecret, BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;
    let secret = aws::db_secret(&authorized_user.sdk_config, &db.name, &db.env).await;
    match &secret {
        Ok(_) => {
            ingest_log(
                &axiom.0,
                &authorized_user.id,
                Action::FetchCredentials(db.name, db.env),
                None,
                None,
            )
            .await
        }
        Err(err) => {
            ingest_log(
                &axiom.0,
                &authorized_user.id,
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
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;
    if let Some(job) = async_task_tracker
        .0
        .lock()
        .await
        .proxies_handlers
        .remove(arn)
    {
        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::StopJob(
                shared::arn_to_name(arn).to_owned(),
                Env::from_any(arn).to_owned(),
                shared::arn_resource_type(arn).unwrap_or_log(),
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
            &authorized_user.id,
            Action::StopJob(
                shared::arn_to_name(arn).to_owned(),
                Env::from_any(arn).to_owned(),
                shared::arn_resource_type(arn).to_owned().unwrap_or_log(),
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
    cluster_cache: tauri::State<'_, ClusterCache>,
    service_cache: tauri::State<'_, ServiceCache>,
    service_details_cache: tauri::State<'_, ServiceDetailsCache>,
    database_cache: tauri::State<'_, DatabaseCache>,
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
        database_cache.0.lock().await.clear()
    }
    {
        cluster_cache.0.lock().await.clear();
    }
    {
        service_cache.0.lock().await.clear();
    }
    {
        service_details_cache.0.lock().await.clear();
    }

    app_state.active_profile = None;
    app_state.sdk_config = None;
    app_state.last_auth_check = 0;

    Ok(())
}

struct WindowNotifier {
    window: Window,
}

impl aws::OnLogFound for WindowNotifier {
    fn notify(&self, logs: Vec<aws::LogEntry>) {
        let _ = self.window.emit("new-log-found", logs);
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
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    {
        let handler = &async_task_tracker.0.lock().await.search_log_handler;
        if let Some(handler) = handler {
            ingest_log(
                &axiom.0,
                &authorized_user.id,
                Action::AbortSearchLogs("new-search-request".to_owned()),
                None,
                None,
            )
            .await;
            handler.abort()
        }
    }
    {
        async_task_tracker.0.lock().await.search_log_handler = None;
    }

    ingest_log(
        &axiom.0,
        &authorized_user.id,
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
            &authorized_user.sdk_config,
            env,
            app,
            start,
            end,
            filter,
            notifier.clone(),
        )
        .await;

        match &result {
            Ok(cnt) => ingest_log(&axiom, &authorized_user.id, action, None, Some(*cnt)).await,
            Err(err) => {
                ingest_log(
                    &axiom,
                    &authorized_user.id,
                    action,
                    Some(err.message.to_owned()),
                    None,
                )
                .await
            }
        }
    }));

    return Ok(());
}

#[tauri::command]
async fn abort_find_logs(
    reason: String,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<(), BError> {
    info!("Attempt to abort find logs: {}", &reason);
    let mut tracker = async_task_tracker.0.lock().await;
    if let Some(handler) = &tracker.search_log_handler {
        handler.abort();
        ingest_log(
            &axiom.0,
            &user_config.0.lock().await.id,
            Action::AbortSearchLogs(reason),
            None,
            None,
        )
        .await;
    }
    tracker.search_log_handler = None;

    Ok(())
}

#[tauri::command]
async fn clusters(
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, ClusterCache>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<Vec<aws::Cluster>, BError> {
    let _ = get_authorized(&app_state.0, &axiom.0).await;

    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;
    let clusters = aws::clusters(&authorized_user.sdk_config, Arc::clone(&cache.0)).await;
    Ok(clusters)
}

#[tauri::command]
async fn services(
    cluster: Cluster,
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, ServiceCache>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<Vec<aws::EcsService>, BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;
    let services = aws::service(&authorized_user.sdk_config, cluster, Arc::clone(&cache.0)).await;
    Ok(services)
}

#[tauri::command]
async fn databases(
    env: shared::Env,
    app_state: tauri::State<'_, AppContextState>,
    cache: tauri::State<'_, DatabaseCache>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<Vec<aws::DbInstance>, BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;
    let databases = aws::databases(&authorized_user.sdk_config, Arc::clone(&cache.0)).await;
    Ok(databases.into_iter().filter(|db| db.env == env).collect())
}

#[tauri::command]
async fn discover(
    name: &str,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
    database_cache: tauri::State<'_, DatabaseCache>,
    service_cache: tauri::State<'_, ServiceCache>,
    cluster_cache: tauri::State<'_, ClusterCache>,
) -> Result<Vec<String>, BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    let name = &name.to_lowercase();
    let tracked_names: HashSet<String>;
    {
        tracked_names = user_config.0.lock().await.tracked_names.clone()
    }

    let mut found_names = HashSet::new();
    if name.len() < 1 {
        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::Discover(name.to_owned()),
            None,
            Some(0),
        )
        .await;
        return Ok(Vec::new());
    }

    {
        let found_dbs: Vec<DbInstance> =
            aws::databases(&authorized_user.sdk_config, Arc::clone(&database_cache.0))
                .await
                .into_iter()
                .filter(|db| {
                    db.arn.contains(name) && !tracked_names.contains(&rds_arn_to_name(&db.arn))
                })
                .collect();

        found_names.extend(found_dbs.into_iter().map(|d| rds_arn_to_name(&d.arn)));
    }

    {
        let clusters =
            aws::clusters(&authorized_user.sdk_config, Arc::clone(&cluster_cache.0)).await;
        let services = aws::services(
            &authorized_user.sdk_config,
            clusters,
            Arc::clone(&service_cache.0),
        )
        .await;
        found_names.extend(
            services
                .values()
                .flatten()
                .filter(|s| {
                    s.arn.contains(name) && !tracked_names.contains(&ecs_arn_to_name(&s.arn))
                })
                .map(|service| ecs_arn_to_name(&service.arn)),
        )
    }

    ingest_log(
        &axiom.0,
        &authorized_user.id,
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
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    let local_port = user_config.0.lock().await.get_db_port(&db.arn);

    let bastions = aws::bastions(&authorized_user.sdk_config).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == db.env)
        .expect("No bastion found");

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::StartRdsProxy(db.name.to_owned(), db.env.clone()),
        None,
        None,
    )
    .await;

    start_aws_ssm_proxy(
        db.arn,
        window,
        bastion.instance_id,
        authorized_user.profile.to_owned(),
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
    axiom: tauri::State<'_, AxiomClientState>,
    database_cache: tauri::State<'_, DatabaseCache>,
    cluster_cache: tauri::State<'_, ClusterCache>,
    service_cache: tauri::State<'_, ServiceCache>,
    service_details_cache: tauri::State<'_, ServiceDetailsCache>,
) -> Result<(), BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;
    {
        cluster_cache.0.lock().await.clear();
    }
    {
        service_cache.0.lock().await.clear();
    }
    {
        service_details_cache.0.lock().await.clear();
    }
    {
        database_cache.0.lock().await.clear();
    }

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::ClearCache,
        None,
        None,
    )
    .await;

    {
        aws::databases(&authorized_user.sdk_config, Arc::clone(&database_cache.0)).await;
    }

    let clusters: Vec<Cluster>;
    {
        clusters = aws::clusters(&authorized_user.sdk_config, Arc::clone(&cluster_cache.0)).await;
    }

    {
        aws::services(
            &authorized_user.sdk_config,
            clusters,
            Arc::clone(&service_cache.0),
        )
        .await;
    }

    window.emit("cache-refreshed", ()).unwrap_or_log();
    Ok(())
}

#[tauri::command]
async fn service_details(
    app: String,
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    databases_cache: tauri::State<'_, DatabaseCache>,
    cluster_cache: tauri::State<'_, ClusterCache>,
    service_cache: tauri::State<'_, ServiceCache>,
    service_details_cache: tauri::State<'_, ServiceDetailsCache>,
) -> Result<(), BError> {
    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::ServiceDetails(app.to_owned()),
        None,
        None,
    )
    .await;
    info!("Called service_details: {}", &app);
    let database_cache = Arc::clone(&databases_cache.0);
    let cluster_cache = Arc::clone(&cluster_cache.0);
    let service_cache = Arc::clone(&service_cache.0);
    let service_details_cache = Arc::clone(&service_details_cache.0);
    tokio::task::spawn(async move {
        info!("Fetching details for: {}", &app);
        let mut dbs_list: Vec<aws::DbInstance> = vec![];
        {
            let databases = aws::databases(&authorized_user.sdk_config, database_cache).await;
            for db in databases.into_iter() {
                if &shared::rds_arn_to_name(&db.arn) == &app {
                    dbs_list.push(db.clone())
                }
            }
        }
        let clusters: Vec<Cluster>;
        {
            clusters = aws::clusters(&authorized_user.sdk_config, cluster_cache).await;
        }

        let service_arns: Vec<String>;
        {
            let services = aws::services(
                &authorized_user.sdk_config,
                clusters.clone(),
                Arc::clone(&service_cache),
            )
            .await;
            service_arns = services
                .values()
                .flatten()
                .filter(|service| app.eq(&service.name))
                .map(|service| service.arn.clone())
                .collect()
        }

        let services: Vec<ServiceDetails> = aws::service_details(
            authorized_user.sdk_config,
            service_arns,
            service_details_cache,
            false,
        )
        .await;

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

    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    let bastions = aws::bastions(&authorized_user.sdk_config).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == service.env)
        .expect("No bastion found");
    let host = format!("{}.service", service.name);
    ingest_log(
        &axiom.0,
        &authorized_user.id,
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
        authorized_user.profile.clone(),
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

    let authorized_user = get_authorized(&app_state.0, &axiom.0).await;

    let dbeaver_path: String;
    {
        dbeaver_path = user_config
            .0
            .lock()
            .await
            .dbeaver_path
            .as_ref()
            .expect("DBeaver needs to be configured")
            .clone();
    }

    let db_secret = aws::db_secret(&authorized_user.sdk_config, &db.name, &db.env).await;
    if let Err(err) = db_secret {
        ingest_log(
            &axiom.0,
            &authorized_user.id,
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
        &authorized_user.id,
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
                ProxyEventMessage::new(arn.clone(), "START".into(), access_port),
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
                ProxyEventMessage::new(arn.clone(), "END".into(), access_port),
            )
            .unwrap_or_log();
    });
}

#[tokio::main]
async fn main() {
    fix_path_env::fix().unwrap_or_log();

    let user = UserConfig::default();
    let user_id = user.id.clone();
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
        .manage(AppContextState(Arc::new(Mutex::new(AppContext {
            active_profile: None,
            user_id: user_id,
            last_auth_check: 0,
            sdk_config: None,
        }))))
        .manage(AsyncTaskManager(Arc::new(Mutex::new(TaskTracker {
            aws_resource_refresher: None,
            proxies_handlers: HashMap::new(),
            search_log_handler: None,
        }))))
        .manage(DatabaseCache(Arc::new(Mutex::new(Vec::new()))))
        .manage(ClusterCache(Arc::new(Mutex::new(Vec::new()))))
        .manage(ServiceCache(Arc::new(Mutex::new(HashMap::new()))))
        .manage(ServiceDetailsCache(Arc::new(Mutex::new(HashMap::new()))))
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

#[derive(Clone)]
struct AppContext {
    active_profile: Option<String>,
    user_id: uuid::Uuid,
    sdk_config: Option<aws_config::SdkConfig>,
    last_auth_check: i64,
}

#[derive(Clone)]
struct AppContextState(Arc<Mutex<AppContext>>);

struct AxiomClientState(Arc<Mutex<Option<axiom_rs::Client>>>);

struct UserConfigState(Arc<Mutex<UserConfig>>);

struct AsyncTaskManager(Arc<Mutex<TaskTracker>>);

struct ServiceDetailsCache(Arc<Mutex<HashMap<String, ServiceDetails>>>);
struct ServiceCache(Arc<Mutex<HashMap<Cluster, Vec<EcsService>>>>);
struct ClusterCache(Arc<Mutex<Vec<Cluster>>>);

struct DatabaseCache(Arc<Mutex<Vec<DbInstance>>>);

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
    StopJob(String, Env, ResourceType),
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
    AbortSearchLogs(String),
}

async fn check_login_and_trigger(
    user_id: &uuid::Uuid,
    profile: &str,
    config: &aws_config::SdkConfig,
    axiom: &Arc<Mutex<Option<axiom_rs::Client>>>,
) -> Result<(), BError> {
    if !aws::is_logged(&config).await {
        info!("Trigger log in into AWS");
        Command::new("aws")
            .args(["sso", "login", "--profile", profile])
            .output()
            .expect("failed to execute process");
        if !aws::is_logged(&config).await {
            ingest_log(
                &axiom,
                user_id,
                Action::LoginCheck(profile.to_owned()),
                Some(String::from("Failed to log in.")),
                None,
            )
            .await;
            return Err(BError::new("login", "Failed to log in"));
        } else {
            ingest_log(
                &axiom,
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
    info!(
        "Ingesting log: {:?}, err: {:?}, record cound: {:?}",
        &action, &error_message, &record_count
    );
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
