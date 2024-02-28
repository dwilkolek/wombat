// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use aws::{Cluster, DbSecret, LogEntry, RdsInstance, ServiceDetails};
use aws_config::BehaviorVersion;
use axiom_rs::Client;
use chrono::{DateTime, Utc};
use cluster_resolver::ClusterResolver;
use dotenvy::dotenv;
use ecs_resolver::EcsResolver;
use libsql::Database;
use log::{debug, error, info, warn};
use rds_resolver::RdsResolver;
use regex::Regex;
use serde_json::json;
use shared::{
    arn_resource_type, arn_to_name, ecs_arn_to_name, rds_arn_to_name, BError, Env, ResourceType,
};
use shared_child::SharedChild;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs};
use tauri::Window;
use tokio::sync::{Mutex, RwLock};
use tracing_unwrap::{OptionExt, ResultExt};
use urlencoding::encode;
use user::UserConfig;
use wait_timeout::ChildExt;

mod aws;
mod cache_db;
mod cluster_resolver;
mod ecs_resolver;
mod global_db;
mod proxy;
mod proxy_authenticators;
mod rds_resolver;
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
    proxy_auth_config: Option<global_db::ProxyAuthConfig>,
}
impl ProxyEventMessage {
    fn new(
        arn: String,
        status: String,
        port: u16,
        proxy_auth_config: Option<global_db::ProxyAuthConfig>,
    ) -> Self {
        Self {
            arn: arn.clone(),
            status,
            port,
            name: arn_to_name(&arn),
            env: Env::from_any(&arn),
            proxy_type: arn_resource_type(&arn).unwrap_or_log(),
            proxy_auth_config,
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct ServiceDetailsPayload {
    app: String,
    services: Vec<aws::ServiceDetails>,
    dbs: Vec<aws::RdsInstance>,
    timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
struct AuthorizedUser {
    id: uuid::Uuid,
    profile: String,
    sdk_config: aws_config::SdkConfig,
}

const CHECK_AUTH_AFTER: i64 = 45 * 60 * 1000; // every 45 minutes.

async fn get_authorized(
    window: &Window,
    app_state: &Arc<Mutex<AppContext>>,
    axiom: &Arc<Mutex<Option<axiom_rs::Client>>>,
) -> Result<AuthorizedUser, String> {
    let mut app_ctx = app_state.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log().clone();
    let user_id = app_ctx.user_id.clone();
    let last_check = app_ctx.last_auth_check;

    let now = chrono::Local::now().timestamp_millis();
    if now - last_check > CHECK_AUTH_AFTER {
        info!("Checking authentication");
        let config = app_ctx.sdk_config.as_ref().unwrap_or_log().clone();
        {
            let login_check = check_login_and_trigger(&user_id, &profile, &config, &axiom).await;
            if login_check.is_err() {
                app_ctx.no_of_failed_logins += 1;
                if app_ctx.no_of_failed_logins == 2 {
                    let _ = window.emit("KILL_ME", "".to_owned());
                }
                return Err("Authentication failed".to_owned());
            }
        }
        app_ctx.no_of_failed_logins = 0;
        app_ctx.last_auth_check = now;
    }

    return Ok(AuthorizedUser {
        profile: profile.to_owned(),
        id: app_ctx.user_id.clone(),
        sdk_config: app_ctx
            .sdk_config
            .as_ref()
            .expect("Sdk Config should be initialized at all times")
            .clone(),
    });
}

#[tauri::command]
async fn user_config(user_config: tauri::State<'_, UserConfigState>) -> Result<UserConfig, BError> {
    let user_config = user_config.0.lock().await;
    Ok(user_config.clone())
}

#[tauri::command]
async fn ping() -> Result<(), ()> {
    info!("Ping");
    Ok(())
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
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    cluster_resolver_instance: tauri::State<'_, ClusterResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,

    database: tauri::State<'_, DatabaseInstance>,
) -> Result<UserConfig, BError> {
    {
        let mut app_state = app_state.0.lock().await;
        app_state.active_profile = Some(profile.to_owned());
        app_state.sdk_config = Some(
            aws_config::defaults(BehaviorVersion::latest())
                .profile_name(profile)
                .load()
                .await,
        );
    }

    let db = database.0.lock().await;
    let _ = db.sync().await;
    let conn = db.connect().unwrap();
    if !global_db::is_feature_enabled(&conn, "wombat").await {
        return Err(BError::new("login", "Not allowed to start."));
    }

    let _ = window.emit("message", "Authenticating...");
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("login", msg)),
        Ok(authorized_user) => authorized_user,
    };

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::Login(profile.to_owned()),
        None,
        None,
    )
    .await;
    let _ = window.emit("message", "Updating profile...");
    let mut user_config = user_config.0.lock().await;
    user_config.use_profile(profile);

    let _ = window.emit("message", "Fetching databases...");
    {
        let mut rds_resolver_instance = rds_resolver_instance.0.write().await;
        let databases = rds_resolver_instance
            .databases(&authorized_user.sdk_config)
            .await;
        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::RefreshRdsList,
            None,
            Some(databases.len()),
        )
        .await;
    }

    let _ = window.emit("message", "Fetching clusters...");
    let clusters;
    {
        let mut cluster_resolver_instance = cluster_resolver_instance.0.write().await;
        clusters = cluster_resolver_instance
            .clusters(&authorized_user.sdk_config)
            .await;
        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::RefreshClusterList,
            None,
            Some(clusters.len()),
        )
        .await;
    }

    let _ = window.emit("message", "Fetching services...");
    {
        let mut ecs_resolver_instance = ecs_resolver_instance.0.write().await;
        let services = ecs_resolver_instance
            .services(&authorized_user.sdk_config, clusters)
            .await;

        ingest_log(
            &axiom.0,
            &authorized_user.id,
            Action::RefreshEcsList,
            None,
            Some(services.len()),
        )
        .await;
    }

    let refresher_axiom = Arc::clone(&axiom.0);
    let refresher_user_id = authorized_user.id.clone();
    let authorized_user = authorized_user.clone();

    let rds_resolver_instance = Arc::clone(&rds_resolver_instance.0);
    let cluster_resolver_instance = Arc::clone(&cluster_resolver_instance.0);
    let ecs_resolver_instance = Arc::clone(&ecs_resolver_instance.0);

    let _ = window.emit("message", "Setting refresh jobs...");
    task_tracker.0.lock().await.aws_resource_refresher = Some(tokio::task::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        let initial_wait = tokio::time::sleep(Duration::from_secs(10));
        initial_wait.await;
        loop {
            interval.tick().await;
            {
                let mut rds_resolver_instance = rds_resolver_instance.write().await;
                let databases = rds_resolver_instance
                    .refresh(&authorized_user.sdk_config)
                    .await;

                if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                    ingest_log_with_client(
                        axiom,
                        &refresher_user_id,
                        Action::RefreshRdsList,
                        None,
                        Some(databases.len()),
                    )
                    .await;
                }
            }

            let clusters;
            {
                {
                    let mut cluster_resolver_instance = cluster_resolver_instance.write().await;

                    clusters = cluster_resolver_instance
                        .refresh(&authorized_user.sdk_config)
                        .await;
                }
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
                let mut ecs_resolver_instance = ecs_resolver_instance.write().await;
                let services = ecs_resolver_instance
                    .refresh(&authorized_user.sdk_config, clusters)
                    .await;

                if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                    ingest_log_with_client(
                        axiom,
                        &authorized_user.id,
                        Action::RefreshEcsList,
                        None,
                        Some(services.len()),
                    )
                    .await;
                }
            }
        }
    }));

    let _ = window.emit("message", "Success!");

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
async fn set_logs_dir_path(
    logs_dir: &str,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    ingest_log(
        &axiom.0,
        &user_config.id,
        Action::SetLogsDirPath(logs_dir.to_owned()),
        None,
        None,
    )
    .await;
    user_config.set_logs_path(logs_dir)
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

async fn db_credentials(
    authorized_user: &AuthorizedUser,
    user_config: &tauri::State<'_, UserConfigState>,
    db: &aws::RdsInstance,
) -> Result<DbSecret, BError> {
    let user_config = user_config.0.lock().await;
    let ssm_role = user_config.ssm_role.as_ref().unwrap_or_log();
    let infra_default_role = &db.name;
    let profile_name = ssm_role.get(&db.name).unwrap_or(&infra_default_role);
    info!("Using infra profile: {}", &profile_name);
    let db_infra_sdk = aws_config::defaults(BehaviorVersion::latest())
        .profile_name(profile_name)
        .load()
        .await;

    let secret = aws::db_secret(&db_infra_sdk, &db.name, &db.env).await;

    if secret.is_err() {
        warn!("Falling back to user profile: {}", &authorized_user.profile);
        return aws::db_secret(&authorized_user.sdk_config, &db.name, &db.env).await;
    }
    return secret;
}

#[tauri::command]
async fn credentials(
    window: Window,
    db: aws::RdsInstance,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<DbSecret, BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("credentials", msg)),
        Ok(authorized_user) => authorized_user,
    };

    let secret = db_credentials(&authorized_user, &user_config, &db).await;
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
    window: Window,
    arn: &str,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("stop_job", msg)),
        Ok(authorized_user) => authorized_user,
    };
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

    app_state.active_profile = None;
    app_state.sdk_config = None;
    app_state.last_auth_check = 0;

    Ok(())
}

struct WindowNotifier {
    window: Window,
}

impl aws::OnLogFound for WindowNotifier {
    fn notify(&mut self, logs: Vec<aws::LogEntry>) {
        let _ = self.window.emit("new-log-found", logs);
    }
    fn success(&mut self) {
        let _ = self.window.emit("find-logs-success", ());
    }
    fn error(&mut self, msg: String) {
        let _ = self.window.emit("find-logs-error", msg);
    }
}

struct FileNotifier {
    window: Window,
    writer: BufWriter<fs::File>,
    filename_location: String,
}

impl aws::OnLogFound for FileNotifier {
    fn notify(&mut self, logs: Vec<aws::LogEntry>) {
        let writer = &mut self.writer;
        let mut data = "".to_owned();
        for log in logs.iter() {
            let log_str = serde_json::to_string(log).unwrap_or_log();
            data.push_str(&log_str);
            data.push_str("\n")
        }
        let _ = writer.write(data.as_bytes());
        if let Some(log) = logs.first() {
            let now = SystemTime::now();
            let timestamp = now.duration_since(UNIX_EPOCH).unwrap_or_log();
            let _ = self.window.emit(
                "new-log-found",
                vec![LogEntry {
                    log_stream_name: log.log_stream_name.to_owned(),
                    ingestion_time: log.ingestion_time,
                    timestamp: timestamp.as_millis() as i64,
                    message: format!("INFO Stored {} logs", logs.len()),
                }],
            );
        }
    }
    fn success(&mut self) {
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap_or_log();
        let _ = self.window.emit(
            "new-log-found",
            vec![LogEntry {
                log_stream_name: "-".to_owned(),
                ingestion_time: timestamp.as_millis() as i64,
                timestamp: timestamp.as_millis() as i64,
                message: format!("TRACE File: {}", self.filename_location),
            }],
        );
        let _ = self.window.emit("find-logs-success", ());
        let _ = self.writer.flush();
    }
    fn error(&mut self, msg: String) {
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap_or_log();
        let _ = self.window.emit(
            "new-log-found",
            vec![LogEntry {
                log_stream_name: "-".to_owned(),
                ingestion_time: timestamp.as_millis() as i64,
                timestamp: timestamp.as_millis() as i64,
                message: format!("ERROR {}", msg),
            }],
        );
        let _ = self.window.emit("find-logs-error", msg);
        let _ = self.writer.flush();
    }
}

#[tauri::command]
async fn find_logs(
    window: Window,
    apps: Vec<String>,
    env: Env,
    start: i64,
    end: i64,
    filter: String,
    filename: Option<String>,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("find_logs", msg)),
        Ok(authorized_user) => authorized_user,
    };

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
        Action::StartSearchLogs(
            apps.to_owned(),
            env.clone(),
            filter.to_string(),
            end - start,
            filename.is_some(),
        ),
        None,
        None,
    )
    .await;

    let axiom = Arc::clone(&axiom.0);
    let user_config = Arc::clone(&user_config.0);
    async_task_tracker.0.lock().await.search_log_handler = Some(tokio::task::spawn(async move {
        let action = Action::SearchLogs(
            apps.clone(),
            env.clone(),
            filter.to_string(),
            end - start,
            filename.is_some(),
        );
        let result = aws::find_logs(
            &authorized_user.sdk_config,
            env,
            apps,
            start,
            end,
            match filter.is_empty() {
                true => None,
                false => Some(filter),
            },
            match &filename {
                None => Arc::new(Mutex::new(WindowNotifier { window })),
                Some(filename) => {
                    let now = SystemTime::now();
                    let timestamp = now.duration_since(UNIX_EPOCH).unwrap_or_log();

                    let user = user_config.lock().await;
                    let logs_dir = user.logs_dir.as_ref().unwrap_or_log().clone();
                    fs::create_dir_all(&logs_dir).unwrap_or_log();
                    let pathbuf =
                        logs_dir.join(format!("{}-{}.log", filename, timestamp.as_millis()));
                    let file = fs::File::create(pathbuf.clone()).unwrap_or_log();

                    Arc::new(Mutex::new(FileNotifier {
                        window,
                        writer: BufWriter::new(file),
                        filename_location: format!(
                            "{}",
                            fs::canonicalize(&pathbuf).unwrap_or_log().display()
                        ),
                    }))
                }
            },
            match &filename.is_some() {
                false => Some(2500),
                true => None,
            },
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
    cache_resolver_instance: tauri::State<'_, ClusterResolverInstance>,
) -> Result<Vec<aws::Cluster>, BError> {
    let cache_resolver_instance = cache_resolver_instance.0.read().await;

    let clusters = cache_resolver_instance.read_clusters().await;
    Ok(clusters)
}

#[tauri::command]
async fn services(
    cluster: Cluster,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
) -> Result<Vec<aws::EcsService>, BError> {
    let ecs_resolver_instance = ecs_resolver_instance.0.read().await;
    let services = ecs_resolver_instance.read_services().await;
    Ok(services
        .into_iter()
        .filter(|s| s.cluster_arn == cluster.arn)
        .collect())
}

#[tauri::command]
async fn databases(
    env: shared::Env,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
) -> Result<Vec<aws::RdsInstance>, BError> {
    let rds_resolver_instance = rds_resolver_instance.0.read().await;
    let databases = rds_resolver_instance.read_databases().await;
    Ok(databases.into_iter().filter(|db| db.env == env).collect())
}

#[tauri::command]
async fn discover(
    window: Window,
    name: &str,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
) -> Result<Vec<String>, BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("discover", msg)),
        Ok(authorized_user) => authorized_user,
    };
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
        let rds_resolver_instance = rds_resolver_instance.0.read().await;
        let found_dbs: Vec<RdsInstance> = rds_resolver_instance
            .read_databases()
            .await
            .into_iter()
            .filter(|db| {
                db.arn.contains(name) && !tracked_names.contains(&rds_arn_to_name(&db.arn))
            })
            .collect();

        found_names.extend(found_dbs.into_iter().map(|d| rds_arn_to_name(&d.arn)));
    }

    {
        let ecs_resolver_instance = ecs_resolver_instance.0.read().await;
        let services = ecs_resolver_instance.read_services().await;

        found_names.extend(
            services
                .into_iter()
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
    db: aws::RdsInstance,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("start_db_proxy", msg)),
        Ok(authorized_user) => authorized_user,
    };

    let local_port = user_config.0.lock().await.get_db_port(&db.arn);
    window
        .emit(
            "proxy-starting",
            ProxyEventMessage::new(db.arn.clone(), "STARTING".into(), local_port.clone(), None),
        )
        .unwrap_or_log();
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

    proxy::start_aws_ssm_proxy(
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
        None,
    )
    .await;

    Ok(())
}

#[tauri::command]
async fn refresh_cache(
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    cluster_resolver_instance: tauri::State<'_, ClusterResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,

    database: tauri::State<'_, DatabaseInstance>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("refresh_cache", msg)),
        Ok(authorized_user) => authorized_user,
    };
    let clusters;
    {
        let mut cluster_resolver_instance = cluster_resolver_instance.0.write().await;
        clusters = cluster_resolver_instance
            .refresh(&authorized_user.sdk_config)
            .await;
    }
    {
        let mut ecs_resolver_instance = ecs_resolver_instance.0.write().await;
        ecs_resolver_instance
            .refresh(&authorized_user.sdk_config, clusters)
            .await;
    }
    {
        let mut rds_resolver_instance = rds_resolver_instance.0.write().await;
        rds_resolver_instance
            .refresh(&authorized_user.sdk_config)
            .await;
    }

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::ClearCache,
        None,
        None,
    )
    .await;

    let db = database.0.lock().await;
    let _ = db.sync().await;
    window.emit("cache-refreshed", ()).unwrap_or_log();
    Ok(())
}

#[tauri::command]
async fn service_details(
    window: Window,
    app: String,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("service_details", msg)),
        Ok(authorized_user) => authorized_user,
    };

    ingest_log(
        &axiom.0,
        &authorized_user.id,
        Action::ServiceDetails(app.to_owned()),
        None,
        None,
    )
    .await;
    info!("Called for service_details: {}", &app);
    let ecs_resolver_instance = Arc::clone(&ecs_resolver_instance.0);
    let rds_resolver_instance = Arc::clone(&rds_resolver_instance.0);
    tokio::task::spawn(async move {
        let mut dbs_list: Vec<aws::RdsInstance> = Vec::new();
        {
            let rds_resolver_instance = rds_resolver_instance.read().await;
            let all_databases = rds_resolver_instance.read_databases().await;
            dbs_list.extend(
                all_databases
                    .into_iter()
                    .filter(|rds| rds.appname_tag == app),
            );
        }

        let service_arns: Vec<String>;
        {
            let ecs_resolver_instance = ecs_resolver_instance.read().await;
            let services = ecs_resolver_instance.read_services().await;
            service_arns = services
                .into_iter()
                .filter(|service| app.eq(&service.name))
                .map(|service| service.arn.clone())
                .collect()
        }

        let services: Vec<ServiceDetails> =
            aws::service_details(authorized_user.sdk_config, service_arns).await;

        window
            .emit(
                "new-service-details",
                ServiceDetailsPayload {
                    app,
                    services,
                    dbs: dbs_list,
                    timestamp: Utc::now(),
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
    proxy_auth_config: Option<global_db::ProxyAuthConfig>,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<(), BError> {
    let local_port = user_config.0.lock().await.get_service_port(&service.arn);

    let aws_local_port = local_port + 10000;
    window
        .emit(
            "proxy-starting",
            ProxyEventMessage::new(
                service.arn.clone(),
                "STARTING".into(),
                aws_local_port,
                proxy_auth_config.clone(),
            ),
        )
        .unwrap_or_log();

    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("start_service_proxy", msg)),
        Ok(authorized_user) => authorized_user,
    };

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

    let mut interceptors: Vec<Box<dyn proxy::ProxyInterceptor>> =
        vec![Box::new(proxy::StaticHeadersInterceptor {
            path_prefix: String::from(""),
            headers: HashMap::from([(String::from("Host"), host.clone())]),
        })];

    if let Some(proxy_auth_config) = proxy_auth_config.as_ref() {
        match proxy_auth_config.auth_type.as_str() {
            "jepsen" => {
                info!("Adding jepsen auth interceptor, {:?}", &proxy_auth_config);
                interceptors.push(Box::new(
                    proxy_authenticators::JepsenAutheticator::from_proxy_auth_config(
                        &authorized_user.sdk_config,
                        proxy_auth_config.clone(),
                    ),
                ));
            }
            "basic" => {
                info!("Adding basic auth interceptor, {:?}", &proxy_auth_config);
                interceptors.push(Box::new(
                    proxy_authenticators::BasicAutheticator::from_proxy_auth_config(
                        &authorized_user.sdk_config,
                        proxy_auth_config.clone(),
                    )
                    .await,
                ));
            }
            _ => {
                warn!("Unknown proxy auth type: {}", proxy_auth_config.auth_type);
            }
        }
    }

    let handle = proxy::start_proxy_to_aws_proxy(
        local_port,
        aws_local_port,
        Arc::new(Mutex::new(proxy::RequestHandler { interceptors })),
    )
    .await;

    proxy::start_aws_ssm_proxy(
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
        proxy_auth_config.clone(),
    )
    .await;

    info!("Started proxy to {}", &service.name);

    Ok(())
}

#[tauri::command]
async fn log_filters(
    database: tauri::State<'_, DatabaseInstance>,
) -> Result<Vec<global_db::LogFilter>, BError> {
    let db = database.0.lock().await;
    let conn = db.connect().unwrap_or_log();
    let filters = global_db::log_filters(&conn).await;

    return Ok(filters);
}
#[tauri::command]
async fn proxy_auth_configs(
    database: tauri::State<'_, DatabaseInstance>,
) -> Result<Vec<global_db::ProxyAuthConfig>, BError> {
    let db = database.0.lock().await;
    let conn = db.connect().unwrap_or_log();
    let configs = global_db::get_proxy_auth_configs(&conn).await;

    return Ok(configs);
}

#[tauri::command]
async fn is_user_feature_enabled(
    feature: &str,
    database: tauri::State<'_, DatabaseInstance>,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<bool, BError> {
    let user_id = format!("{}", &user_config.0.lock().await.id);
    let db = database.0.lock().await;
    let conn = db.connect().unwrap_or_log();
    let is_enabled = global_db::is_user_feature_enabled(&conn, feature, user_id.as_str()).await;

    return Ok(is_enabled);
}

#[tauri::command]
async fn open_dbeaver(
    window: Window,
    db: aws::RdsInstance,
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

    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("open_dbeaver", msg)),
        Ok(authorized_user) => authorized_user,
    };

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

    let secret = db_credentials(&authorized_user, &user_config, &db).await;
    if let Err(err) = secret {
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
    let db_secret = secret.unwrap_or_log();
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

async fn initialize_database_connection() -> Database {
    let db_file = env::var("DB_PATH").unwrap_or_else(|_| {
        debug!("Using default db path since DB_PATH was not set");
        user::wombat_dir()
            .join("wombat.db")
            .to_str()
            .unwrap_or_log()
            .to_string()
    });

    let auth_token = env::var("TURSO_AUTH_TOKEN").unwrap_or_else(|_| {
        debug!("Using default token since TURSO_AUTH_TOKEN was not set");
        "%%TURSO_AUTH_TOKEN%%".to_string()
    });

    let url = env::var("TURSO_SYNC_URL")
        .unwrap_or_else(|_| {
            debug!("Using default sync url since TURSO_SYNC_URL was not set");
            "%%TURSO_SYNC_URL%%".to_string()
        })
        .replace("libsql", "https");

    //  info!("Database file={}, remote={}", &db_file, &url);

    let db = libsql::Builder::new_remote_replica(db_file, url, auth_token)
        .periodic_sync(Duration::from_secs(3600))
        .build()
        .await
        .unwrap();

    let _ = db.sync().await.unwrap();

    info!("Database ref created");

    let conn = db.connect().unwrap();

    info!("Database connected");

    info!("Database synced");
    global_db::migrate(&conn).await;
    let _ = db.sync().await.unwrap();

    return db;
}

async fn initialize_axiom(user: &UserConfig) -> AxiomClientState {
    let client = Client::builder()
        .with_token("%%AXIOM_TOKEN%%")
        .with_org_id("%%AXIOM_ORG%%")
        .build();
    return match format!("%%{}%%", "AXIOM_TOKEN") == "%%AXIOM_TOKEN%%" {
        true => AxiomClientState(Arc::new(Mutex::new(None))),
        false => AxiomClientState(Arc::new(Mutex::new(match client {
            Ok(client) => {
                ingest_log_with_client(&client, &user.id, Action::Start, None, None).await;
                Some(client)
            }
            Err(_) => None,
        }))),
    };
}

async fn initialize_cache_db() -> libsql::Database {
    return libsql::Builder::new_local(
        user::wombat_dir()
            .join("cache.db")
            .to_str()
            .unwrap_or_log()
            .to_string(),
    )
    .build()
    .await
    .unwrap();
}

#[tokio::main]
async fn main() {
    fix_path_env::fix().unwrap_or_log();
    let _ = dotenv();
    let logger = env::var("LOGGER").unwrap_or_else(|_| "file".to_string());

    let _guard = match logger.as_str() {
        "console" => {
            tracing_subscriber::fmt().init();
            None
        }
        "file" => {
            let file_appender = tracing_appender::rolling::daily(
                user::wombat_dir().join("logs").as_path(),
                "wombat",
            );
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            tracing_subscriber::fmt().with_writer(non_blocking).init();
            Some(guard)
        }
        _ => {
            panic!("Unknown logger: {}", logger);
        }
    };
    let db = Arc::new(Mutex::new(initialize_database_connection().await));
    let cache_db = Arc::new(RwLock::new(initialize_cache_db().await));

    let user = UserConfig::default();
    tauri::Builder::default()
        .manage(initialize_axiom(&user).await)
        .manage(AppContextState(Arc::new(Mutex::new(AppContext {
            active_profile: None,
            user_id: user.id.clone(),
            last_auth_check: 0,
            sdk_config: None,
            no_of_failed_logins: 0,
        }))))
        .manage(UserConfigState(Arc::new(Mutex::new(user))))
        .manage(AsyncTaskManager(Arc::new(Mutex::new(TaskTracker {
            aws_resource_refresher: None,
            proxies_handlers: HashMap::new(),
            search_log_handler: None,
        }))))
        .manage(DatabaseInstance(db.clone()))
        .manage(RdsResolverInstance(Arc::new(RwLock::new(
            RdsResolver::new(cache_db.clone()).await,
        ))))
        .manage(ClusterResolverInstance(Arc::new(RwLock::new(
            ClusterResolver::new(cache_db.clone()).await,
        ))))
        .manage(EcsResolverInstance(Arc::new(RwLock::new(
            EcsResolver::new(cache_db.clone()).await,
        ))))
        .invoke_handler(tauri::generate_handler![
            user_config,
            set_dbeaver_path,
            set_logs_dir_path,
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
            abort_find_logs,
            log_filters,
            proxy_auth_configs,
            is_user_feature_enabled,
            ping
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
    no_of_failed_logins: i64,
}

#[derive(Clone)]
struct AppContextState(Arc<Mutex<AppContext>>);

struct AxiomClientState(Arc<Mutex<Option<axiom_rs::Client>>>);

struct UserConfigState(Arc<Mutex<UserConfig>>);

struct AsyncTaskManager(Arc<Mutex<TaskTracker>>);

struct RdsResolverInstance(Arc<RwLock<RdsResolver>>);
struct ClusterResolverInstance(Arc<RwLock<ClusterResolver>>);
struct EcsResolverInstance(Arc<RwLock<EcsResolver>>);

struct DatabaseInstance(Arc<Mutex<Database>>);

struct TaskTracker {
    aws_resource_refresher: Option<tokio::task::JoinHandle<()>>,
    proxies_handlers: HashMap<String, Arc<SharedChild>>,
    search_log_handler: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HomeEntry {
    tracked_name: shared::TrackedName,
    services: HashMap<String, aws::ServiceDetails>,
    dbs: Vec<aws::RdsInstance>,
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
    RefreshEcsList,
    RefreshClusterList,
    Message(String),
    ClearCache,
    UpdateTrackedNames(String),
    SetDbeaverPath(String),
    SetLogsDirPath(String),
    SetPrefferedEnvs(Vec<Env>),
    Discover(String),
    Logout(String),
    StartSearchLogs(Vec<String>, Env, String, i64, bool),
    SearchLogs(Vec<String>, Env, String, i64, bool),
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
        let mut child = Command::new("aws")
            .args(["sso", "login", "--profile", profile])
            .spawn()
            .expect("failed to execute process");

        let one_sec = Duration::from_secs(30);
        let _ = match child.wait_timeout(one_sec).unwrap() {
            Some(status) => status.code(),
            None => {
                child.kill().unwrap();
                child.wait().unwrap().code()
            }
        };

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
        "Ingesting log: {:?}, err: {:?}, record count: {:?}",
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
