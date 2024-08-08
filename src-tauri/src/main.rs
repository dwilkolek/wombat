// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use aws::{Cluster, DbSecret, InfraProfile, LogEntry, RdsInstance, SsoProfile};
use chrono::{DateTime, Utc};
use cluster_resolver::ClusterResolver;
#[cfg(debug_assertions)]
use dotenvy::dotenv;
use ecs_resolver::EcsResolver;
use log::{error, info, warn};
use rds_resolver::RdsResolver;
use sha2::{Digest, Sha256};
use shared::{arn_to_name, BError, BrowserExtension, CookieJar, Env};
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

mod aws;
mod cache_db;
mod cluster_resolver;
mod dependency_check;
mod ecs_resolver;
mod proxy;
mod proxy_authenticators;
mod rds_resolver;
mod rest_api;
mod shared;
mod user;
mod wombat_api;

#[derive(Clone, serde::Serialize)]
struct TaskKilled {
    arn: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NewTaskParams {
    port: u16,
    proxy_auth_config: Option<wombat_api::ProxyAuthConfig>,
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
    profile: String,
}

const CHECK_AUTH_AFTER: i64 = 5 * 60 * 1000; // every 5 minutes.

async fn get_authorized(
    window: &Window,
    app_state: &Arc<Mutex<AppContext>>,
) -> Result<AuthorizedUser, String> {
    let mut app_ctx = app_state.lock().await;
    let profile = app_ctx.active_profile.as_ref().unwrap_or_log().clone();
    let last_check = app_ctx.last_auth_check;
    let aws_config_provider = app_ctx.aws_config_provider.clone();
    let now = chrono::Local::now().timestamp_millis();
    if now - last_check > CHECK_AUTH_AFTER {
        info!("Checking authentication");
        for env in profile.sso_profiles.keys() {
            let aws_config_provider = aws_config_provider.read().await;
            let (aws_profile, config) = aws_config_provider.sso_config(env).await;
            let login_check = check_login_and_trigger(&aws_profile, &config).await;
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
    } else {
        info!("Checking authentication skipped");
    }

    Ok(AuthorizedUser {
        profile: profile.name.clone(),
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
async fn chrome_extension_dir() -> Result<String, ()> {
    Ok(user::chrome_extension_dir()
        .into_os_string()
        .into_string()
        .unwrap())
}
#[tauri::command]
async fn browser_extension_health(
    browser_ext_instance: tauri::State<'_, BrowserExtensionInstance>,
) -> Result<shared::BrowserExtensionStatus, ()> {
    let browser_ext = browser_ext_instance.0.lock().await;
    Ok(browser_ext.to_status())
}

#[tauri::command]
async fn cookie_jar_status(
    cookie_jar: tauri::State<'_, CookieJarInstance>,
) -> Result<shared::CookieJarStatus, ()> {
    let cookie_jar = cookie_jar.0.lock().await;
    Ok(cookie_jar.to_status())
}

#[tauri::command]
async fn favorite(
    name: &str,
    user_config: tauri::State<'_, UserConfigState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    let aws_config_provider = aws_config_provider.0.read().await;
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
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("login", msg));
    };

    let _ = window.emit("message", "Updating profile...");
    let mut user_config = user_config.0.lock().await;
    user_config.use_profile(profile, environments, tracked_names);

    let cache_db = Arc::new(RwLock::new(initialize_cache_db(profile).await));

    let _ = window.emit("message", "Fetching databases...");
    {
        let mut rds_resolver_instance = rds_resolver_instance.0.write().await;
        rds_resolver_instance.init(cache_db.clone()).await;
        rds_resolver_instance.databases().await;
    }

    let _ = window.emit("message", "Fetching clusters...");
    let clusters;
    {
        let mut cluster_resolver_instance = cluster_resolver_instance.0.write().await;
        cluster_resolver_instance.init(cache_db.clone()).await;
        clusters = cluster_resolver_instance.clusters().await;
    }

    let _ = window.emit("message", "Fetching services...");
    {
        let mut ecs_resolver_instance = ecs_resolver_instance.0.write().await;
        ecs_resolver_instance.init(cache_db.clone()).await;
        ecs_resolver_instance.services(clusters).await;
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
    api.report_versions(None).await;

    let rds_resolver_instance = Arc::clone(&rds_resolver_instance.0);
    let cluster_resolver_instance = Arc::clone(&cluster_resolver_instance.0);
    let ecs_resolver_instance = Arc::clone(&ecs_resolver_instance.0);

    let _ = window.emit("message", "Setting refresh jobs...");
    task_tracker.0.lock().await.aws_resource_refresher = Some(tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        {
            let mut rds_resolver_instance = rds_resolver_instance.write().await;
            rds_resolver_instance.refresh().await;
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
        let clusters;
        {
            {
                let mut cluster_resolver_instance = cluster_resolver_instance.write().await;

                clusters = cluster_resolver_instance.refresh().await;
            }
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
        {
            let mut ecs_resolver_instance = ecs_resolver_instance.write().await;
            ecs_resolver_instance.refresh(clusters).await;
        }
    }));

    let _ = window.emit("message", "Success!");

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
async fn set_logs_dir_path(
    logs_dir: &str,
    user_config: tauri::State<'_, UserConfigState>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    user_config.set_logs_path(logs_dir)
}

#[tauri::command]
async fn save_preffered_envs(
    envs: Vec<shared::Env>,
    user_config: tauri::State<'_, UserConfigState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<UserConfig, BError> {
    let mut user_config = user_config.0.lock().await;
    let aws_config_provider = aws_config_provider.0.read().await;
    user_config.save_preffered_envs(&aws_config_provider.active_wombat_profile.name, envs)
}
#[tauri::command]
async fn kv_put(value: String, kv_store: tauri::State<'_, KVStoreInstance>) -> Result<String, ()> {
    let mut kv_store = kv_store.0.lock().await;
    Ok(kv_store.put(value))
}

#[tauri::command]
async fn kv_get(
    key: String,
    kv_store: tauri::State<'_, KVStoreInstance>,
) -> Result<String, String> {
    let kv_store = kv_store.0.lock().await;
    match kv_store.get(&key) {
        Some(v) => Ok(v),
        None => Err(format!("Key {} not found", &key)),
    }
}

#[tauri::command]
async fn kv_delete(key: String, kv_store: tauri::State<'_, KVStoreInstance>) -> Result<String, ()> {
    let mut kv_store = kv_store.0.lock().await;
    kv_store.delete(&key);
    Ok(key)
}

#[tauri::command]
async fn credentials(
    window: Window,
    db: aws::RdsInstance,
    app_state: tauri::State<'_, AppContextState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<DbSecret, BError> {
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("credentials", msg));
    };

    let aws_config_provider = aws_config_provider.0.read().await;
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

    secret
}

#[tauri::command]
async fn stop_job(
    window: Window,
    arn: &str,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("stop_job", msg));
    };
    let mut tracker = async_task_tracker.0.lock().await;
    if let Some(handle) = tracker.task_handlers.remove(arn) {
        let kill_result = handle.send(());
        if kill_result.is_ok() {
            info!("Killing dependant job, success: {}", kill_result.is_ok());
            let _ = window.emit(
                "task-killed",
                TaskKilled {
                    arn: arn.to_owned(),
                },
            );
        }
    }
    if let Some(job) = tracker.proxies_handlers.remove(arn) {
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
    }

    Ok(())
}

#[tauri::command]
async fn logout(
    app_state: tauri::State<'_, AppContextState>,
    task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    let mut app_state = app_state.0.lock().await;

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

impl aws::LogSearchMonitor for WindowNotifier {
    fn notify(&mut self, logs: Vec<aws::LogEntry>) {
        let _ = self.window.emit("new-log-found", logs);
    }
    fn success(&mut self, msg: String) {
        let _ = self.window.emit("find-logs-success", msg);
    }
    fn error(&mut self, msg: String) {
        let _ = self.window.emit("find-logs-error", msg);
    }
    fn message(&mut self, msg: String) {
        let _ = self.window.emit("find-logs-message", msg);
    }
}

struct FileNotifier {
    window: Window,
    writer: BufWriter<fs::File>,
    filename_location: String,
}

impl aws::LogSearchMonitor for FileNotifier {
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
    fn success(&mut self, msg: String) {
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
        let _ = self.window.emit("find-logs-success", msg);
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
    fn message(&mut self, msg: String) {
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap_or_log();
        let _ = self.window.emit(
            "new-log-found",
            vec![LogEntry {
                log_stream_name: "-".to_owned(),
                ingestion_time: timestamp.as_millis() as i64,
                timestamp: timestamp.as_millis() as i64,
                message: format!("INFO {}", msg.clone()),
            }],
        );
        let _ = self.window.emit("find-logs-message", msg);
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
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    user_config: tauri::State<'_, UserConfigState>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<(), BError> {
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("find_logs", msg));
    };

    {
        let handler = &async_task_tracker.0.lock().await.search_log_handler;
        if let Some(handler) = handler {
            handler.abort()
        }
    }
    {
        async_task_tracker.0.lock().await.search_log_handler = None;
    }

    let user_config = Arc::clone(&user_config.0);
    let sdk_config: aws_config::SdkConfig;
    {
        let aws_config_provider = aws_config_provider.0.read().await;
        //TODO: it will be annoying to do to search logs with different sdk_configs...
        //Let's hope it will keep working.
        let app_config = aws_config_provider.sso_config(&env).await;
        sdk_config = app_config.1
    }
    async_task_tracker.0.lock().await.search_log_handler = Some(tokio::task::spawn(async move {
        let _ = aws::find_logs(
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
                false => Some(10000),
                true => None,
            },
        )
        .await;
    }));

    Ok(())
}

#[tauri::command]
async fn abort_find_logs(
    reason: String,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<(), BError> {
    info!("Attempt to abort find logs: {}", &reason);
    let mut tracker = async_task_tracker.0.lock().await;
    if let Some(handler) = &tracker.search_log_handler {
        handler.abort();
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
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<String, BError> {
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("restart_service", msg));
    };

    let aws_config_provider = aws_config_provider.0.read().await;
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
#[allow(clippy::too_many_arguments)]
async fn discover(
    window: Window,
    name: &str,
    app_state: tauri::State<'_, AppContextState>,
    user_config: tauri::State<'_, UserConfigState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
) -> Result<Vec<String>, BError> {
    let authorized_user = match get_authorized(&window, &app_state.0).await {
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
        return Ok(Vec::new());
    }

    {
        let rds_resolver_instance = rds_resolver_instance.0.read().await;
        let found_dbs: Vec<RdsInstance> = rds_resolver_instance
            .read_databases()
            .await
            .into_iter()
            .filter(|db| db.arn.contains(name) && !tracked_names.contains(&arn_to_name(&db.arn)))
            .collect();

        found_names.extend(found_dbs.into_iter().map(|d| arn_to_name(&d.arn)));
    }

    {
        let ecs_resolver_instance = ecs_resolver_instance.0.read().await;
        let services = ecs_resolver_instance.read_services().await;

        found_names.extend(
            services
                .into_iter()
                .filter(|s| s.arn.contains(name) && !tracked_names.contains(&arn_to_name(&s.arn)))
                .map(|service| arn_to_name(&service.arn)),
        )
    }

    Ok(found_names.into_iter().collect())
}

#[tauri::command]
async fn refresh_cache(
    window: Window,
    app_state: tauri::State<'_, AppContextState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    cluster_resolver_instance: tauri::State<'_, ClusterResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
) -> Result<(), BError> {
    {
        let mut app_state = app_state.0.lock().await;
        app_state.no_of_failed_logins = 0;
        app_state.last_auth_check = 0;
    }

    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("refresh_cache", msg));
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

    window.emit("cache-refreshed", ()).unwrap_or_log();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn service_details(
    window: Window,
    app: String,
    app_state: tauri::State<'_, AppContextState>,
    rds_resolver_instance: tauri::State<'_, RdsResolverInstance>,
    ecs_resolver_instance: tauri::State<'_, EcsResolverInstance>,
    rate_limitter: tauri::State<'_, ServiceDetailsRateLimiter>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<(), BError> {
    let authorized_user = match get_authorized(&window, &app_state.0).await {
        Err(msg) => return Err(BError::new("service_details", msg)),
        Ok(authorized_user) => authorized_user,
    };

    let environments: Vec<Env>;
    {
        let aws_config_provider = aws_config_provider.0.read().await;
        environments = aws_config_provider.configured_envs();
    }

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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn start_db_proxy(
    window: Window,
    db: aws::RdsInstance,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<NewTaskParams, BError> {
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("start_db_proxy", msg));
    };

    let aws_config_provider = aws_config_provider.0.read().await;
    let (aws_profile, aws_config) = aws_config_provider
        .app_config(&db.normalized_name, &db.env)
        .await
        .expect("Missing sdk_config to start db proxy");
    let region = aws_config_provider.get_region(&aws_profile).await;

    let mut user_config = user_config.0.lock().await;
    let local_port = user_config.get_db_port(&db.arn);

    let bastions = aws::bastions(&aws_config).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == db.env)
        .expect("No bastion found");

    let proxy_started = proxy::start_aws_ssm_proxy(
        db.arn.clone(),
        window.clone(),
        bastion.instance_id,
        aws_profile,
        region,
        db.endpoint.address,
        db.endpoint.port,
        local_port,
        None,
        local_port,
        async_task_tracker.clone(),
    )
    .await;

    match proxy_started {
        Ok(port) => Ok(NewTaskParams {
            port,
            proxy_auth_config: None,
        }),
        Err(proxy_err) => Err(BError::new("start_db_proxy", format!("{proxy_err}"))),
    }
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn start_service_proxy(
    window: Window,
    service: aws::EcsService,
    infra_profile: Option<InfraProfile>,
    sso_profile: Option<SsoProfile>,
    headers: HashMap<String, String>,
    proxy_auth_config: Option<wombat_api::ProxyAuthConfig>,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
    aws_config_provider: tauri::State<'_, AwsConfigProviderInstance>,
) -> Result<NewTaskParams, BError> {
    let local_port;
    {
        let mut user_config = user_config.0.lock().await;
        local_port = user_config.get_service_port(&service.arn);
    }

    let aws_local_port = local_port + 10000;
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("start_service_proxy", msg));
    };
    let aws_config_provider = aws_config_provider.0.read().await;
    let (aws_profile, aws_config) = aws_config_provider
        .with_dev_way_check(&infra_profile, &sso_profile)
        .await
        .expect("Missing sdk_config to start service proxy");

    let bastions = aws::bastions(&aws_config).await;
    let bastion = bastions
        .into_iter()
        .find(|b| b.env == service.env)
        .expect("No bastion found");

    let mut interceptors: Vec<Box<dyn proxy::ProxyInterceptor>> =
        vec![Box::new(proxy::StaticHeadersInterceptor {
            path_prefix: String::from(""),
            headers,
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
                    proxy_authenticators::BasicAuthenticator::from_proxy_auth_config(
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

    let handle = proxy::start_proxy_to_adress(
        local_port,
        format!("http://localhost:{}/", aws_local_port).to_owned(),
        Arc::new(RwLock::new(proxy::RequestHandler { interceptors })),
    )
    .await;

    let region = aws_config_provider.get_region(&aws_profile).await;

    let host = format!("{}.service", service.name);
    let proxy_started = proxy::start_aws_ssm_proxy(
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
    )
    .await;

    info!(
        "Proxy to {} started={}",
        &service.name,
        proxy_started.is_ok()
    );
    match proxy_started {
        Ok(port) => Ok(NewTaskParams {
            port,
            proxy_auth_config,
        }),
        Err(proxy_err) => Err(BError::new("start_ecs_proxy", format!("{proxy_err}"))),
    }
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn start_lambda_app_proxy(
    app: String,
    env: shared::Env,
    address: String,
    headers: HashMap<String, String>,
    cookie_jar: tauri::State<'_, CookieJarInstance>,
    user_config: tauri::State<'_, UserConfigState>,
    async_task_tracker: tauri::State<'_, AsyncTaskManager>,
) -> Result<NewTaskParams, BError> {
    let local_port;
    {
        let mut user_config = user_config.0.lock().await;
        local_port = user_config.get_lambda_app_port(&app, &env);
    }
    let lambda_arn = format!("lambdaApp::{app}::{env}");

    let mut interceptors: Vec<Box<dyn proxy::ProxyInterceptor>> =
        vec![Box::new(proxy::StaticHeadersInterceptor {
            path_prefix: String::from(""),
            headers,
        })];

    interceptors.push(Box::new(proxy_authenticators::CookieAutheticator {
        env,
        jar: cookie_jar.0.clone(),
    }));

    let handle = proxy::start_proxy_to_adress(
        local_port,
        address.clone(),
        Arc::new(RwLock::new(proxy::RequestHandler { interceptors })),
    )
    .await;

    async_task_tracker
        .0
        .lock()
        .await
        .task_handlers
        .insert(lambda_arn.clone(), handle);

    info!("Started lambda proxy={} to {}", lambda_arn, &address);

    Ok(NewTaskParams {
        port: local_port,
        proxy_auth_config: None,
    })
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
    let aws_config_provider = aws_config_provider.0.read().await;
    Ok(dependency_check::check_dependencies(&mut wombat_api, &aws_config_provider).await)
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
#[allow(clippy::too_many_arguments)]
async fn open_dbeaver(
    window: Window,
    db: aws::RdsInstance,
    port: u16,
    user_config: tauri::State<'_, UserConfigState>,
    app_state: tauri::State<'_, AppContextState>,
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
    if let Err(msg) = get_authorized(&window, &app_state.0).await {
        return Err(BError::new("open_dbeaver", msg));
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
    let aws_config_provider = aws_config_provider.0.read().await;
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

    let db_secret = match secret {
        Ok(secret) => secret,
        Err(error) => {
            error!("failed to get rds secret, reason={}", &error.message);
            return Err(error);
        }
    };

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
    }
}

#[cfg(not(debug_assertions))]
fn app_config() -> AppConfig {
    AppConfig {
        logger: "file".to_owned(),
        wombat_api_url: "%%WOMBAT_API_URL%%".to_string(),
        wombat_api_user: "%%WOMBAT_API_USER%%".to_string(),
        wombat_api_password: "%%WOMBAT_API_PASSWORD%%".to_string(),
    }
}

#[derive(Debug)]
struct AppConfig {
    logger: String,
    wombat_api_url: String,
    wombat_api_user: String,
    wombat_api_password: String,
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
    let user = UserConfig::default();
    let cookie_jar = Arc::new(Mutex::new(CookieJar {
        cookies: Vec::new(),
    }));
    let browser_ext = Arc::new(Mutex::new(BrowserExtension {
        last_health_check: Utc::now() - chrono::Duration::days(100),
        version: None,
        reported_version: None,
    }));
    let wombat_api = wombat_api::WombatApi::new(
        app_config.wombat_api_url.clone(),
        app_config.wombat_api_user.clone(),
        app_config.wombat_api_password.clone(),
        user.id,
    );

    let wombat_api = Arc::new(RwLock::new(wombat_api));

    tokio::task::spawn(rest_api::serve(
        cookie_jar.clone(),
        browser_ext.clone(),
        wombat_api.clone(),
    ));

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

    let aws_config_provider = Arc::new(RwLock::new(aws::AwsConfigProvider::new().await));
    let app = tauri::Builder::default()
        .setup(|app| {
            let resource_path = app
                .path_resolver()
                .resolve_resource("../chrome-extension")
                .expect("failed to chrome_extension resource");
            let chrome_extension_dir = user::chrome_extension_dir();
            if chrome_extension_dir.exists() {
                let _ = fs::remove_dir(&chrome_extension_dir);
            }

            let cope_result = shared::copy_dir_all(resource_path, chrome_extension_dir);
            if cope_result.is_err() {
                warn!("Chrome extension copy failed, reason: {:?}", cope_result)
            } else {
                info!("Chrome extension copy sucessful")
            }

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
        .manage(AppContextState(Arc::new(Mutex::new(AppContext {
            active_profile: None,
            aws_config_provider: aws_config_provider.clone(),
            last_auth_check: 0,
            no_of_failed_logins: 0,
        }))))
        .manage(BrowserExtensionInstance(browser_ext))
        .manage(CookieJarInstance(cookie_jar))
        .manage(UserConfigState(Arc::new(Mutex::new(user))))
        .manage(AsyncTaskManager(Arc::new(Mutex::new(TaskTracker {
            aws_resource_refresher: None,
            proxies_handlers: HashMap::new(),
            task_handlers: HashMap::new(),
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
        .manage(WombatApiInstance(wombat_api))
        .manage(KVStoreInstance(Arc::new(Mutex::new(KVStore {
            store: HashMap::new(),
        }))))
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
            wombat_aws_profiles,
            start_lambda_app_proxy,
            chrome_extension_dir,
            browser_extension_health,
            cookie_jar_status,
            kv_put,
            kv_get,
            kv_delete
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
    last_auth_check: i64,
    no_of_failed_logins: i64,
}

struct AppContextState(Arc<Mutex<AppContext>>);

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
struct BrowserExtensionInstance(Arc<Mutex<BrowserExtension>>);
struct CookieJarInstance(Arc<Mutex<CookieJar>>);
struct KVStoreInstance(Arc<Mutex<KVStore>>);

struct KVStore {
    store: HashMap<String, String>,
}
impl KVStore {
    fn put(&mut self, value: String) -> String {
        let key = format!("{:X}", Sha256::new().chain_update(&value).finalize());
        self.store.insert(key.clone(), value);
        key
    }

    fn delete(&mut self, key: &str) {
        self.store.remove(key);
    }

    fn get(&self, key: &str) -> Option<String> {
        self.store.get(key).map(|x| x.to_owned())
    }
}

struct TaskTracker {
    aws_resource_refresher: Option<tokio::task::JoinHandle<()>>,
    proxies_handlers: HashMap<String, Arc<SharedChild>>,
    task_handlers: HashMap<String, tokio::sync::oneshot::Sender<()>>,
    search_log_handler: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HomeEntry {
    tracked_name: shared::TrackedName,
    services: HashMap<String, aws::ServiceDetails>,
    dbs: Vec<aws::RdsInstance>,
}

async fn check_login_and_trigger(
    profile: &str,
    config: &aws_config::SdkConfig,
) -> Result<(), BError> {
    if !aws::is_logged(profile, config).await {
        info!("Trigger log in into AWS");
        aws::cli_login(profile);
        // let mut child = Command::new("aws")
        //     .args(["sso", "login", "--profile", profile])
        //     .spawn()
        //     .expect("failed to execute process");

        // let one_sec = Duration::from_secs(30);
        // let _ = match child.wait_timeout(one_sec).unwrap() {
        //     Some(status) => status.code(),
        //     None => {
        //         child.kill().unwrap();
        //         child.wait().unwrap().code()
        //     }
        // };

        if !aws::is_logged(profile, config).await {
            return Err(BError::new("login", "Failed to log in"));
        } else {
            return Ok(());
        }
    }
    Ok(())
}
