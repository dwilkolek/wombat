// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use aws::{Cluster, DbSecret, InfraProfile, LogEntry, RdsInstance, SsoProfile};
use axiom_rs::Client;
use chrono::{DateTime, Utc};
use cluster_resolver::ClusterResolver;
#[cfg(debug_assertions)]
use dotenvy::dotenv;
use ecs_resolver::EcsResolver;
use log::{error, info, warn};
use rds_resolver::RdsResolver;
use serde_json::json;
use shared::{
    arn_resource_type, arn_to_name, ecs_arn_to_name, rds_arn_to_name, BError, Env, ResourceType,
};
use shared_child::SharedChild;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
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
mod dependency_check;
mod ecs_resolver;
mod proxy;
mod proxy_authenticators;
mod rds_resolver;
mod shared;
mod user;
mod wombat_api;

#[derive(Clone, serde::Serialize)]
struct ProxyEventMessage {
    arn: String,
    status: String,
    port: u16,
    name: String,
    env: Env,
    proxy_type: ResourceType,
    proxy_auth_config: Option<wombat_api::ProxyAuthConfig>,
}
impl ProxyEventMessage {
    fn new(
        arn: String,
        status: String,
        port: u16,
        proxy_auth_config: Option<wombat_api::ProxyAuthConfig>,
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
    services: Vec<Result<aws::ServiceDetails, aws::ServiceDetailsMissing>>,
    dbs: Vec<aws::RdsInstance>,
    timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
struct AuthorizedUser {
    id: uuid::Uuid,
    profile: String,
}

const CHECK_AUTH_AFTER: i64 = 45 * 60 * 1000; // every 45 minutes.

async fn get_authorized(
    window: &Window,
    app_state: &Arc<Mutex<AppContext>>,
    axiom: &Arc<Mutex<Option<axiom_rs::Client>>>,
) -> Result<AuthorizedUser, String> {
    let mut app_ctx = app_state.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log().clone();
    let user_id = app_ctx.user_id;
    let last_check = app_ctx.last_auth_check;
    let aws_config_provider = app_ctx.aws_config_provider.clone();
    let now = chrono::Local::now().timestamp_millis();
    if now - last_check > CHECK_AUTH_AFTER {
        info!("Checking authentication");
        for env in profile.sso_profiles.keys() {
            let mut aws_config_provider = aws_config_provider.write().await;
            let (aws_profile, config) = aws_config_provider.sso_config(env).await;
            let login_check = check_login_and_trigger(user_id, &aws_profile, &config, axiom).await;
            if login_check.is_err() {
                app_ctx.no_of_failed_logins += 1;
                if app_ctx.no_of_failed_logins == 2 {
                    let _ = window.emit("KILL_ME", "".to_owned());
                }
                return Err("Authentication failed".to_owned());
            } else {
                app_ctx.no_of_failed_logins = 0;
            }
        }
        app_ctx.no_of_failed_logins = 0;
        app_ctx.last_auth_check = now;
    }

    Ok(AuthorizedUser {
        profile: profile.name.clone(),
        id: app_ctx.user_id,
    })
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
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    let aws_config_provider = aws_config_provider.0.read().await;
    ingest_log(
        &axiom.0,
        user_config.id,
        Action::UpdateTrackedNames(name.to_owned()),
        None,
        None,
    )
    .await;

    user_config.favorite(
        &aws_config_provider.active_wombat_profile.name,
        name.to_owned(),
    )
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn login(
    profile: &str,
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    axiom: tauri::State<'_, AxiomClientState>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,

    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    cluster_resolver_instance: tauri::State<'_, ClusterResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,

    wombat_api_instance: tauri::State<'_, WombatApiInstance>,
) -> Result<UserConfig, BError> {
    let environments: Vec<Env>;
    let tracked_names: HashSet<String>;
    {
        let mut aws_config_provider = aws_config_provider.0.write().await;

        let wombat_api = wombat_api_instance.0.read().await;
        let is_enabled = wombat_api.is_feature_enabled("dev-way").await;
        aws_config_provider.login(profile.to_owned(), is_enabled);

        environments = aws_config_provider.configured_envs();
        tracked_names = aws_config_provider
            .active_wombat_profile
            .sso_profiles
            .values()
            .flat_map(|sso| sso.infra_profiles.clone())
            .map(|infra| infra.app)
            .collect();

        let mut app_state = app_state.0.lock().await;
        app_state.active_profile = Some(aws_config_provider.active_wombat_profile.clone());
    }

    let _ = window.emit("message", "Authenticating...");
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("login", msg)),
        Ok(authorized_user) => authorized_user,
    };

    ingest_log(
        &axiom.0,
        authorized_user.id,
        Action::Login(profile.to_owned()),
        None,
        None,
    )
    .await;

    let _ = window.emit("message", "Updating profile...");
    let mut user_config = user_config.0.lock().await;
    user_config.use_profile(profile, environments, tracked_names);

    let cache_db = Arc::new(RwLock::new(initialize_cache_db(profile).await));

    let _ = window.emit("message", "Fetching databases...");
    {
        let mut rds_resolver_instance = rds_resolver_instance.0.write().await;
        rds_resolver_instance.init(cache_db.clone()).await;
        let databases = rds_resolver_instance.databases().await;
        ingest_log(
            &axiom.0,
            authorized_user.id,
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
        cluster_resolver_instance.init(cache_db.clone()).await;
        clusters = cluster_resolver_instance.clusters().await;
        ingest_log(
            &axiom.0,
            authorized_user.id,
            Action::RefreshClusterList,
            None,
            Some(clusters.len()),
        )
        .await;
    }

    let _ = window.emit("message", "Fetching services...");
    {
        let mut ecs_resolver_instance = ecs_resolver_instance.0.write().await;
        ecs_resolver_instance.init(cache_db.clone()).await;
        let services = ecs_resolver_instance.services(clusters).await;

        ingest_log(
            &axiom.0,
            authorized_user.id,
            Action::RefreshEcsList,
            None,
            Some(services.len()),
        )
        .await;
    }

    let _ = window.emit("message", "Syncing global state...");
    let mut api = wombat_api_instance.0.write().await;
    let api_status = api.status().await;
    if let Err(status) = api_status {
        return Err(BError::new(
            "login",
            format!("Wombat backend API is not ok. Reason: {}", status),
        ));
    }

    let refresher_axiom = Arc::clone(&axiom.0);
    let refresher_user_id = authorized_user.id;
    let authorized_user = authorized_user.clone();

    let rds_resolver_instance = Arc::clone(&rds_resolver_instance.0);
    let cluster_resolver_instance = Arc::clone(&cluster_resolver_instance.0);
    let ecs_resolver_instance = Arc::clone(&ecs_resolver_instance.0);

    let _ = window.emit("message", "Setting refresh jobs...");
    task_tracker.0.lock().await.aws_resource_refresher = Some(tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        {
            let mut rds_resolver_instance = rds_resolver_instance.write().await;
            let databases = rds_resolver_instance.refresh().await;

            if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                ingest_log_with_client(
                    axiom,
                    refresher_user_id,
                    Action::RefreshRdsList,
                    None,
                    Some(databases.len()),
                )
                .await;
            }
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
        let clusters;
        {
            {
                let mut cluster_resolver_instance = cluster_resolver_instance.write().await;

                clusters = cluster_resolver_instance.refresh().await;
            }
            if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                ingest_log_with_client(
                    axiom,
                    authorized_user.id,
                    Action::RefreshClusterList,
                    None,
                    Some(clusters.len()),
                )
                .await;
            }
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
        {
            let mut ecs_resolver_instance = ecs_resolver_instance.write().await;
            let services = ecs_resolver_instance.refresh(clusters).await;

            if let Some(axiom) = refresher_axiom.lock().await.as_ref() {
                ingest_log_with_client(
                    axiom,
                    authorized_user.id,
                    Action::RefreshEcsList,
                    None,
                    Some(services.len()),
                )
                .await;
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
        user_config.id,
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
        user_config.id,
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
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
    axiom: tauri::State<'_, AxiomClientState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    let aws_config_provider = aws_config_provider.0.read().await;
    ingest_log(
        &axiom.0,
        user_config.id,
        Action::SetPrefferedEnvs(envs.clone()),
        None,
        None,
    )
    .await;
    user_config.save_preffered_envs(&aws_config_provider.active_wombat_profile.name, envs)
}

#[tauri::command]
async fn credentials(
    window: Window,
    db: aws::RdsInstance,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<DbSecret, BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("credentials", msg)),
        Ok(authorized_user) => authorized_user,
    };
    let mut aws_config_provider = aws_config_provider.0.write().await;
    let (_, aws_config) = aws_config_provider
        .app_config(&db.normalized_name, &db.env)
        .await
        .expect_or_log("Config doesn't exist");

    let secret;
    let found_db_secret = aws::db_secret(&aws_config, &db.name, &db.env).await;
    match found_db_secret {
        Ok(found_secret) => {
            secret = Ok(found_secret);
        }
        Err(err) => {
            if aws_config_provider.dev_way {
                let (override_aws_profile, override_aws_config) =
                    aws_config_provider.sso_config(&db.env).await;
                warn!("Falling back to user profile: {}", &override_aws_profile);
                secret = aws::db_secret(&override_aws_config, &db.name, &db.env).await;
            } else {
                secret = Err(err)
            }
        }
    }

    match &secret {
        Ok(_) => {
            ingest_log(
                &axiom.0,
                authorized_user.id,
                Action::FetchCredentials(db.name, db.env),
                None,
                None,
            )
            .await
        }
        Err(err) => {
            ingest_log(
                &axiom.0,
                authorized_user.id,
                Action::FetchCredentials(db.name, db.env),
                Some(err.message.clone()),
                None,
            )
            .await
        }
    };
    secret
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
            authorized_user.id,
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

        // TODO: typically user lack permission to terminate session.
        // Most importanly we kill session on user side so probably it's fine.
        // That whole thing bellow is responsible for it.

        // let mut out = job.take_stdout();
        // let mut session_id: Option<String> = None;
        // let session_regex = Regex::new("Starting session with SessionId: (.*)").unwrap_or_log();
        //
        // if let Some(stdout) = &mut out {
        //     let lines = BufReader::new(stdout).lines().enumerate().take(10);
        //     for (_counter, line) in lines {
        //         if let Ok(line) = line {
        //             let captures = session_regex.captures(&line);
        //             let found_session_id = captures
        //                 .and_then(|c| c.get(1))
        //                 .map(|e| e.as_str().to_owned());
        //             if found_session_id.is_some() {
        //                 session_id = found_session_id;
        //                 break;
        //             }
        //         }
        //     }
        // }

        // if let Some(session_id) = session_id {
        //     let app_ctx = app_state.0.lock().await;
        //     let profile = app_ctx.active_profile.as_ref().unwrap_or_log();
        //     let killed_session_output = Command::new("aws")
        //         .args([
        //             "ssm",
        //             "terminate-session",
        //             "--session-id",
        //             &session_id,
        //             "--profile",
        //             &profile,
        //         ])
        //         .output();
        //     match killed_session_output {
        //         Ok(output) => info!("Attempted to kill session in SSM: {:?}", output),
        //         Err(e) => warn!("Failed to kill session in SSM {}", e),
        //     };
        // }
    } else {
        ingest_log(
            &axiom.0,
            authorized_user.id,
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
            user_state.0.lock().await.id,
            Action::Logout(profile.name.clone()),
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
            data.push('\n')
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

#[allow(clippy::too_many_arguments)]
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
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
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
                authorized_user.id,
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
        authorized_user.id,
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
    let sdk_config: aws_config::SdkConfig;
    {
        let mut aws_config_provider = aws_config_provider.0.write().await;
        //TODO: it will be annoying to do to search logs with different sdk_configs...
        //Let's hope it will keep working.
        let app_config = aws_config_provider.sso_config(&env).await;
        sdk_config = app_config.1
    }
    async_task_tracker.0.lock().await.search_log_handler = Some(tokio::task::spawn(async move {
        let action = Action::SearchLogs(
            apps.clone(),
            env.clone(),
            filter.to_string(),
            end - start,
            filename.is_some(),
        );
        let result = aws::find_logs(
            &sdk_config,
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
            Ok(cnt) => ingest_log(&axiom, authorized_user.id, action, None, Some(*cnt)).await,
            Err(err) => {
                ingest_log(
                    &axiom,
                    authorized_user.id,
                    action,
                    Some(err.message.to_owned()),
                    None,
                )
                .await
            }
        }
    }));

    Ok(())
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
            user_config.0.lock().await.id,
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

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn restart_service(
    cluster_arn: String,
    env: Env,
    service_name: String,
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<String, BError> {
    let _authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("restart_service", msg)),
        Ok(authorized_user) => authorized_user,
    };
    let mut aws_config_provider = aws_config_provider.0.write().await;
    let (aws_profile, aws_config) = aws_config_provider
        .app_config_with_fallback(&service_name, &env)
        .await
        .expect("Missing sdk_config to restart service");
    info!(
        "Attemping to restart service {} on {} with profile {}",
        &service_name, &cluster_arn, &aws_profile
    );
    let ecs_resolver_instance = ecs_resolver_instance.0.read().await;
    ecs_resolver_instance
        .restart_service(window, aws_config, cluster_arn, service_name)
        .await
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
        let user_config = &user_config.0.lock().await;
        let preferences = user_config.preferences.as_ref();
        tracked_names = preferences
            .and_then(|prefereces| {
                prefereces
                    .get(&authorized_user.profile)
                    .map(|preferences| preferences.tracked_names.clone())
            })
            .unwrap_or(HashSet::new());
    }

    let mut found_names = HashSet::new();
    if name.is_empty() {
        ingest_log(
            &axiom.0,
            authorized_user.id,
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
        authorized_user.id,
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
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("start_db_proxy", msg)),
        Ok(authorized_user) => authorized_user,
    };
    let mut aws_config_provider = aws_config_provider.0.write().await;
    let (aws_profile, aws_config) = aws_config_provider
        .app_config(&db.normalized_name, &db.env)
        .await
        .expect("Missing sdk_config to start db proxy");

    let mut user_config = user_config.0.lock().await;
    let local_port = user_config.get_db_port(&db.arn);
    window
        .emit(
            "proxy-starting",
            ProxyEventMessage::new(db.arn.clone(), "STARTING".into(), local_port, None),
        )
        .unwrap_or_log();

    let bastions = aws::bastions(&aws_config).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == db.env)
        .expect("No bastion found");

    ingest_log(
        &axiom.0,
        authorized_user.id,
        Action::StartRdsProxy(db.name.to_owned(), db.env.clone()),
        None,
        None,
    )
    .await;
    let region = aws_config_provider.get_region(&aws_profile).await;
    proxy::start_aws_ssm_proxy(
        db.arn,
        window,
        bastion.instance_id,
        aws_profile,
        region,
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
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("refresh_cache", msg)),
        Ok(authorized_user) => authorized_user,
    };
    let clusters;
    {
        let mut cluster_resolver_instance = cluster_resolver_instance.0.write().await;
        clusters = cluster_resolver_instance.refresh().await;
    }
    {
        let mut ecs_resolver_instance = ecs_resolver_instance.0.write().await;
        ecs_resolver_instance.refresh(clusters).await;
    }
    {
        let mut rds_resolver_instance = rds_resolver_instance.0.write().await;
        rds_resolver_instance.refresh().await;
    }

    ingest_log(&axiom.0, authorized_user.id, Action::ClearCache, None, None).await;

    window.emit("cache-refreshed", ()).unwrap_or_log();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn service_details(
    window: Window,
    app: String,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
    rate_limitter: tauri::State<'_, ServiceDetailsRateLimiter>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0, &axiom.0).await {
        Err(msg) => return Err(BError::new("service_details", msg)),
        Ok(authorized_user) => authorized_user,
    };

    let environments: Vec<Env>;
    {
        let aws_config_provider = aws_config_provider.0.read().await;
        environments = aws_config_provider.configured_envs();
    }

    ingest_log(
        &axiom.0,
        authorized_user.id,
        Action::ServiceDetails(app.to_owned()),
        None,
        None,
    )
    .await;

    info!(
        "Called for service_details: {}, profile: {}",
        &app, &authorized_user.profile
    );
    let ecs_resolver_instance = Arc::clone(&ecs_resolver_instance.0);
    let rds_resolver_instance = Arc::clone(&rds_resolver_instance.0);

    let mut dbs_list: Vec<aws::RdsInstance> = Vec::new();
    {
        let rds_resolver_instance = rds_resolver_instance.read().await;
        let all_databases = rds_resolver_instance.read_databases().await;
        dbs_list.extend(
            all_databases
                .into_iter()
                .filter(|rds| rds.appname_tag == app && environments.contains(&rds.env)),
        );
    }

    let services_to_resolve: Vec<aws::EcsService>;
    {
        let ecs_resolver_instance = ecs_resolver_instance.read().await;
        let services = ecs_resolver_instance.read_services().await;
        services_to_resolve = services
            .into_iter()
            .filter(|service| app.eq(&service.name) && environments.contains(&service.env))
            .collect()
    }
    let mut counter = rate_limitter.0.lock().await;
    dbg!(&services_to_resolve);
    let mut services =
        aws::service_details(aws_config_provider.0.clone(), services_to_resolve.clone()).await;
    let mut retry_count = 3;
    while services.iter().any(|s| s.is_err()) && retry_count > 0 {
        retry_count -= 1;
        counter.inc();

        let err_service_arns: Vec<String> = services
            .iter()
            .filter(|s| s.is_err())
            .map(|s| {
                let e = s.as_ref().unwrap_err();
                e.arn.clone()
            })
            .collect();
        warn!(
            "Calling service details for app {} resulted in {} errors. Backing off for 5s. Retries left: {}. Total back offs: {}",
           &app, err_service_arns.len(), &retry_count, &counter.count
        );
        tokio::time::sleep(Duration::from_secs(5)).await;
        let err_services = services_to_resolve
            .iter()
            .filter(|s| err_service_arns.contains(&s.arn))
            .cloned()
            .collect();
        let services_refetched =
            aws::service_details(aws_config_provider.0.clone(), err_services).await;

        services = services
            .into_iter()
            .map(|s| match s {
                Ok(s) => Ok(s),
                Err(s) => services_refetched
                    .iter()
                    .find(|rs| match rs {
                        Ok(rs) => rs.arn == s.arn,
                        Err(rs) => rs.arn == s.arn,
                    })
                    .cloned()
                    .unwrap(),
            })
            .collect();
    }

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
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn start_service_proxy(
    window: Window,
    service: aws::EcsService,
    infra_profile: Option<InfraProfile>,
    sso_profile: Option<SsoProfile>,
    proxy_auth_config: Option<wombat_api::ProxyAuthConfig>,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    axiom: tauri::State<'_, AxiomClientState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<(), BError> {
    let local_port;
    {
        let mut user_config = user_config.0.lock().await;
        local_port = user_config.get_service_port(&service.arn);
    }

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

    let mut aws_config_provider = aws_config_provider.0.write().await;
    let (aws_profile, aws_config) = aws_config_provider
        .with_dev_way_check(&infra_profile, &sso_profile)
        .await
        .expect("Missing sdk_config to start service proxy");
    let bastions = aws::bastions(&aws_config).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == service.env)
        .expect("No bastion found");
    let host = format!("{}.service", service.name);
    ingest_log(
        &axiom.0,
        authorized_user.id,
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
        // let source_app_profile = proxy_auth_config.from_app.clone();
        // let (mut source_app_profile, mut source_app_config) = aws_config_provider
        //     .app_config(&source_app_profile, &service.env)
        //     .await
        //     .expect("Missing sdk_config to setup auth interceptor");
        // if aws_config_provider.dev_way {
        //     (source_app_profile, source_app_config) =
        //         aws_config_provider.sso_config(&service.env).await;
        // }
        //

        let (source_app_profile, source_app_config) = aws_config_provider
            .with_dev_way_check(&infra_profile, &sso_profile)
            .await
            .expect("Missing sdk_config to setup auth interceptor");
        match proxy_auth_config.auth_type.as_str() {
            "jepsen" => {
                info!(
                    "Adding jepsen auth interceptor, profile={}",
                    &source_app_profile
                );
                interceptors.push(Box::new(
                    proxy_authenticators::JepsenAutheticator::from_proxy_auth_config(
                        &source_app_config,
                        proxy_auth_config.clone(),
                    ),
                ));
            }
            "basic" => {
                info!(
                    "Adding basic auth interceptor, profile={}",
                    &source_app_profile
                );
                interceptors.push(Box::new(
                    proxy_authenticators::BasicAutheticator::from_proxy_auth_config(
                        &source_app_config,
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
        Arc::new(RwLock::new(proxy::RequestHandler { interceptors })),
    )
    .await;

    let region = aws_config_provider.get_region(&aws_profile).await;

    proxy::start_aws_ssm_proxy(
        service.arn,
        window,
        bastion.instance_id,
        aws_profile,
        region,
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
    wombat_api_instance: tauri::State<'_, WombatApiInstance>,
) -> Result<Vec<wombat_api::LogFilter>, BError> {
    let wombat_api = wombat_api_instance.0.read().await;
    let filters = wombat_api.log_filters().await;
    Ok(filters)
}

#[tauri::command]
async fn proxy_auth_configs(
    wombat_api_instance: tauri::State<'_, WombatApiInstance>,
) -> Result<Vec<wombat_api::ProxyAuthConfig>, BError> {
    let wombat_api = wombat_api_instance.0.read().await;
    let configs = wombat_api.get_proxy_auth_configs().await;

    Ok(configs)
}

#[tauri::command]
async fn is_feature_enabled(
    feature: &str,
    wombat_api_instance: tauri::State<'_, WombatApiInstance>,
) -> Result<bool, BError> {
    let wombat_api = wombat_api_instance.0.read().await;
    let is_enabled = wombat_api.is_feature_enabled(feature).await;

    Ok(is_enabled)
}

#[tauri::command]
async fn check_dependencies(
    wombat_api_instance: tauri::State<'_, WombatApiInstance>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<HashMap<String, Result<String, String>>, ()> {
    let mut wombat_api = wombat_api_instance.0.write().await;
    let mut aws_config_provider = aws_config_provider.0.write().await;
    Ok(dependency_check::check_dependencies(&mut wombat_api, &mut aws_config_provider).await)
}

#[tauri::command]
async fn available_infra_profiles(
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<Vec<aws::InfraProfile>, BError> {
    let provider = aws_config_provider.0.read().await;
    Ok(provider
        .active_wombat_profile
        .sso_profiles
        .values()
        .flat_map(|sso| sso.infra_profiles.clone())
        .collect())
}

#[tauri::command]
async fn available_sso_profiles(
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<Vec<aws::SsoProfile>, BError> {
    let provider = aws_config_provider.0.read().await;
    Ok(provider
        .active_wombat_profile
        .sso_profiles
        .values()
        .cloned()
        .collect())
}

#[tauri::command]
async fn wombat_aws_profiles(
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<Vec<aws::WombatAwsProfile>, BError> {
    let provider = aws_config_provider.0.read().await;
    Ok(provider.wombat_profiles.clone())
}

#[tauri::command]
async fn open_dbeaver(
    window: Window,
    db: aws::RdsInstance,
    port: u16,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    axiom: tauri::State<'_, AxiomClientState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
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
    let mut aws_config_provider = aws_config_provider.0.write().await;
    let (_, aws_config) = aws_config_provider
        .app_config(&db.normalized_name, &db.env)
        .await
        .expect("Missing sdk_config to get secreto for dbBeaver");

    let secret;
    let found_db_secret = aws::db_secret(&aws_config, &db.name, &db.env).await;
    match found_db_secret {
        Ok(found_secret) => {
            secret = Ok(found_secret);
        }
        Err(_) => {
            if aws_config_provider.dev_way {
                let (override_aws_profile, override_aws_config) =
                    aws_config_provider.sso_config(&db.env).await;
                warn!("Falling back to user profile: {}", &override_aws_profile);
                secret = aws::db_secret(&override_aws_config, &db.name, &db.env).await;
            } else {
                return Err(BError::new("db_secret", "No secret found"));
            }
        }
    }

    if let Err(err) = secret {
        ingest_log(
            &axiom.0,
            authorized_user.id,
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
        authorized_user.id,
        Action::OpenDbeaver(db.name.to_owned(), db.env.clone()),
        None,
        None,
    )
    .await;
    Command::new(dbeaver_path)
        .args([
            "-con",
            &db_beaver_con_parma(
                db.arn.split(':').last().unwrap_or_log(),
                "localhost",
                port,
                &db_secret,
            ),
        ])
        .output()
        .expect("failed to execute process");
    Ok(())
}

#[cfg(debug_assertions)]
fn app_config() -> AppConfig {
    let _ = dotenv();
    AppConfig {
        wombat_api_url: env::var("WOMBAT_API_URL").unwrap_or_else(|_| {
            warn!("Using default token since WOMBAT_API_URL was not set");
            "%%WOMBAT_API_URL%%".to_string()
        }),
        wombat_api_user: env::var("WOMBAT_API_USER").unwrap_or_else(|_| {
            warn!("Using default token since WOMBAT_API_USER was not set");
            "%%WOMBAT_API_USER%%".to_string()
        }),
        wombat_api_password: env::var("WOMBAT_API_PASSWORD").unwrap_or_else(|_| {
            warn!("Using default token since WOMBAT_API_PASSWORD was not set");
            "%%WOMBAT_API_PASSWORD%%".to_string()
        }),
        logger: env::var("LOGGER").unwrap_or_else(|_| "console".to_string()),
        axiom_token: None,
        axiom_org: "shrug".to_owned(),
    }
}

#[cfg(not(debug_assertions))]
fn app_config() -> AppConfig {
    AppConfig {
        logger: "file".to_owned(),
        wombat_api_url: "%%WOMBAT_API_URL%%".to_string(),
        wombat_api_user: "%%WOMBAT_API_USER%%".to_string(),
        wombat_api_password: "%%WOMBAT_API_PASSWORD%%".to_string(),
        axiom_token: Some("%%AXIOM_TOKEN%%".to_string()),
        axiom_org: "%%AXIOM_ORG%%".to_owned(),
    }
}

#[derive(Debug)]
struct AppConfig {
    logger: String,
    wombat_api_url: String,
    wombat_api_user: String,
    wombat_api_password: String,
    axiom_token: Option<String>,
    axiom_org: String,
}

async fn initialize_axiom(user: &UserConfig, congig: &AppConfig) -> AxiomClientState {
    return match congig.axiom_token.is_none() {
        true => AxiomClientState(Arc::new(Mutex::new(None))),
        false => {
            let client = Client::builder()
                .with_token(congig.axiom_token.as_ref().unwrap())
                .with_org_id(congig.axiom_org.as_str())
                .build();
            return AxiomClientState(Arc::new(Mutex::new(match client {
                Ok(client) => {
                    ingest_log_with_client(&client, user.id, Action::Start, None, None).await;
                    Some(client)
                }
                Err(_) => None,
            })));
        }
    };
}

async fn initialize_cache_db(profile: &str) -> libsql::Database {
    if let Ok(paths) = user::wombat_dir().read_dir() {
        for path in paths.flatten() {
            if let Ok(file_name) = path.file_name().into_string() {
                info!("file name: {}", file_name);
                if file_name.starts_with("wombat.db") {
                    info!("deleting wombat db: {}", file_name);
                    let _ = fs::remove_file(path.path());
                }
                if file_name.starts_with("cache-") {
                    info!("deleting cache file: {}", file_name);
                    let _ = fs::remove_file(path.path());
                }
            }
        }
    }

    libsql::Builder::new_local(
        user::wombat_dir()
            .join(format!("v1-cache-{}.db", profile))
            .to_str()
            .unwrap_or_log(),
    )
    .build()
    .await
    .unwrap()
}

#[tokio::main]
async fn main() {
    fix_path_env::fix().unwrap_or_log();

    let app_config = app_config();
    let _guard = match app_config.logger.as_str() {
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
            panic!("Unknown logger: {}", &app_config.logger);
        }
    };

    let cache_db = Arc::new(RwLock::new(initialize_cache_db("default").await));
    let user = UserConfig::default();

    let api = wombat_api::WombatApi::new(
        app_config.wombat_api_url.clone(),
        app_config.wombat_api_user.clone(),
        app_config.wombat_api_password.clone(),
        user.id,
    );
    let aws_config_provider = Arc::new(RwLock::new(aws::AwsConfigProvider::new().await));
    let app = tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();
            tauri::async_runtime::spawn(async move {
                match tauri::updater::builder(handle).check().await {
                    Ok(update) => {
                        // if update.is_update_available() {
                        //     update.download_and_install().await.unwrap();
                        // }
                        info!("update available: {}", update.is_update_available());
                    }
                    Err(e) => {
                        warn!("failed to get update: {}", e);
                    }
                }
            });
            Ok(())
        })
        .manage(initialize_axiom(&user, &app_config).await)
        .manage(AppContextState(Arc::new(Mutex::new(AppContext {
            active_profile: None,
            aws_config_provider: aws_config_provider.clone(),
            user_id: user.id,
            last_auth_check: 0,
            no_of_failed_logins: 0,
        }))))
        .manage(UserConfigState(Arc::new(Mutex::new(user))))
        .manage(AsyncTaskManager(Arc::new(Mutex::new(TaskTracker {
            aws_resource_refresher: None,
            proxies_handlers: HashMap::new(),
            search_log_handler: None,
        }))))
        .manage(AwsConfigProviderInstance(aws_config_provider.clone()))
        .manage(RdsResolverInstance(Arc::new(RwLock::new(
            RdsResolver::new(cache_db.clone(), aws_config_provider.clone()),
        ))))
        .manage(ServiceDetailsRateLimiter(Arc::new(Mutex::new(LockCount {
            count: 0,
        }))))
        .manage(ClusterResolverInstance(Arc::new(RwLock::new(
            ClusterResolver::new(cache_db.clone(), aws_config_provider.clone()),
        ))))
        .manage(EcsResolverInstance(Arc::new(RwLock::new(
            EcsResolver::new(cache_db.clone(), aws_config_provider.clone()),
        ))))
        .manage(WombatApiInstance(Arc::new(RwLock::new(api))))
        .invoke_handler(tauri::generate_handler![
            user_config,
            set_dbeaver_path,
            set_logs_dir_path,
            save_preffered_envs,
            login,
            logout,
            clusters,
            services,
            restart_service,
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
            is_feature_enabled,
            ping,
            check_dependencies,
            available_infra_profiles,
            available_sso_profiles,
            wombat_aws_profiles
        ])
        .build(tauri::generate_context!())
        .expect("Error while running tauri application");

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Updater(updater_event) = event {
            match updater_event {
                tauri::UpdaterEvent::UpdateAvailable {
                    body,
                    date,
                    version,
                } => {
                    info!("update available {} {:?} {}", body, date, version);
                }
                // Emitted when the download is about to be started.
                tauri::UpdaterEvent::Pending => {
                    info!("update is pending!");
                }
                // tauri::UpdaterEvent::DownloadProgress {
                //     chunk_length,
                //     content_length,
                // } => {
                //     info!("downloaded {} of {:?}", chunk_length, content_length);
                // }
                // Emitted when the download has finished and the update is about to be installed.
                tauri::UpdaterEvent::Downloaded => {
                    info!("update has been downloaded!");
                }
                // Emitted when the update was installed. You can then ask to restart the app.
                tauri::UpdaterEvent::Updated => {
                    info!("app has been updated");
                }
                // Emitted when the app already has the latest version installed and an update is not needed.
                tauri::UpdaterEvent::AlreadyUpToDate => {
                    info!("app is already up to date");
                }
                // Emitted when there is an error with the updater. We suggest to listen to this event even if the default dialog is enabled.
                tauri::UpdaterEvent::Error(error) => {
                    info!("failed to update: {}", error);
                }
                _ => {}
            }
        }
    });
}

struct AppContext {
    active_profile: Option<aws::WombatAwsProfile>,
    aws_config_provider: Arc<RwLock<aws::AwsConfigProvider>>,
    user_id: uuid::Uuid,
    last_auth_check: i64,
    no_of_failed_logins: i64,
}

struct AppContextState(Arc<Mutex<AppContext>>);

struct AxiomClientState(Arc<Mutex<Option<axiom_rs::Client>>>);

struct UserConfigState(Arc<Mutex<UserConfig>>);

pub struct AsyncTaskManager(Arc<Mutex<TaskTracker>>);

struct ServiceDetailsRateLimiter(Arc<Mutex<LockCount>>);
struct LockCount {
    count: i64,
}
impl LockCount {
    fn inc(&mut self) {
        self.count += 1;
    }
}

struct AwsConfigProviderInstance(Arc<RwLock<aws::AwsConfigProvider>>);
struct RdsResolverInstance(Arc<RwLock<RdsResolver>>);
struct ClusterResolverInstance(Arc<RwLock<ClusterResolver>>);
struct EcsResolverInstance(Arc<RwLock<EcsResolver>>);

struct WombatApiInstance(Arc<RwLock<wombat_api::WombatApi>>);

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
    user_id: uuid::Uuid,
    profile: &str,
    config: &aws_config::SdkConfig,
    axiom: &Arc<Mutex<Option<axiom_rs::Client>>>,
) -> Result<(), BError> {
    if !aws::is_logged(config).await {
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

        if !aws::is_logged(config).await {
            ingest_log(
                axiom,
                user_id,
                Action::LoginCheck(profile.to_owned()),
                Some(String::from("Failed to log in.")),
                None,
            )
            .await;
            return Err(BError::new("login", "Failed to log in"));
        } else {
            ingest_log(
                axiom,
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
    user_id: uuid::Uuid,
    action: Action,
    error_message: Option<String>,
    record_count: Option<usize>,
) {
    info!(
        "Ingesting log: {:?}, err: {:?}, record count: {:?}",
        &action, &error_message, &record_count
    );
    if let Some(client) = client.lock().await.as_ref() {
        ingest_log_with_client(client, user_id, action, error_message, record_count).await;
    }
}

async fn ingest_log_with_client(
    client: &Client,
    user_id: uuid::Uuid,
    action: Action,
    error_message: Option<String>,
    record_count: Option<usize>,
) {
    if let Err(e) = client
        .ingest(
            "wombat",
            vec![json!(ActionLog {
                user_id,
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
