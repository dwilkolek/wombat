use crate::{AsyncTaskManager, TaskKilled};
use async_trait::async_trait;
use filepath::FilePath;
use log::{error, info, warn};
use shared_child::SharedChild;
use std::collections::HashMap;
use std::fs::{self, File};
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};
use tempfile::TempDir;
use tokio::time::sleep;
use tracing_unwrap::ResultExt;
use warp::http::{HeaderName, HeaderValue};
use warp::hyper::body::Bytes;
use warp::hyper::Method;
use warp::Filter as WarpFilter;
use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response, Headers};

#[allow(clippy::too_many_arguments)]
pub async fn start_aws_ssm_proxy(
    arn: String,
    app_handle: AppHandle,
    bastion: String,
    profile: String,
    region: String,
    target: String,
    target_port: u16,
    local_port: u16,

    abort_on_exit: Option<tokio::sync::oneshot::Sender<()>>,
    access_port: u16,
    async_task_manager: &tauri::State<'_, AsyncTaskManager>,
) -> Result<u16, ProxyError> {
    let mut command = Command::new("aws");

    command.args([
        "ssm",
        "start-session",
        "--target",
        &bastion,
        "--profile",
        &profile,
        "--region",
        region.as_str(),
        "--document-name",
        "AWS-StartPortForwardingSessionToRemoteHost",
        "--parameters",
        &format!(
            "{{\"host\":[\"{target}\"], \"portNumber\":[\"{target_port}\"], \"localPortNumber\":[\"{local_port}\"]}}",
        ),
    ]);

    info!(
        "Local: {}, target: {}, access: {}",
        &local_port, &target_port, &access_port
    );

    warn!("making sure {} is free", &local_port);
    kill_pid_on_port(local_port);

    let tmp_dir = TempDir::new().unwrap();
    let out_log: File = File::create(tmp_dir.path().join(format!("out-{local_port}.log"))).unwrap();
    let err_log: File = File::create(tmp_dir.path().join(format!("err-{local_port}.log"))).unwrap();

    let out_log_path = format!("{}", out_log.path().unwrap().display());
    let err_log_path = format!("{}", err_log.path().unwrap().display());

    command.stdout(Stdio::from(out_log));
    command.stderr(Stdio::from(err_log));

    info!("Out log path: {out_log_path:?}");
    info!("Error log path: {err_log_path:?}");
    info!("Executnig cmd: {command:?} ");

    let shared_child = SharedChild::spawn(&mut command).unwrap_or_log();

    let shared_child_arc = Arc::new(shared_child);
    let child_arc_clone = shared_child_arc.clone();

    async_task_manager
        .0
        .lock()
        .await
        .proxies_handlers
        .insert(arn.clone(), shared_child_arc);
    let started_at = SystemTime::now();

    loop {
        sleep(Duration::from_millis(100)).await;
        let contents = fs::read_to_string(out_log_path.clone()).unwrap_or_default();

        info!("Output: {contents}");
        if contents.contains("Waiting for connections...") {
            break;
        }
        if SystemTime::now()
            .duration_since(started_at)
            .unwrap()
            .as_secs()
            > 10
        {
            let contents = fs::read_to_string(err_log_path.clone()).unwrap_or_default();
            error!("Failed to start proxy: {contents}");

            kill_pid_on_port(local_port);
            return Err(if contents.contains("Error loading SSO Token") {
                ProxyError::ErrorSsoToken
            } else {
                ProxyError::Unknown
            });
        }
    }
    tokio::task::spawn(async move {
        // {\"host\":[\"$endpoint\"], \"portNumber\":[\"5432\"], \"localPortNumber\":[\"$port\"]}
        // aws ssm start-session \
        //  --target "$instance" \
        //  --profile "$profile" \
        //  --document-name AWS-StartPortForwardingSessionToRemoteHost \
        //  --parameters "$parameters"

        let _ = child_arc_clone.wait();
        if let Some(handle) = abort_on_exit {
            let kill_result = handle.send(());
            info!("Killing dependant job, success: {}", kill_result.is_ok());
        }
        app_handle
            .emit("task-killed", TaskKilled { arn: arn.clone() })
            .unwrap_or_log();
        kill_pid_on_port(local_port);
    });

    Ok(access_port)
}

#[async_trait]
pub trait ProxyInterceptor: Send + Sync {
    fn applies(&self, uri: &str) -> bool;
    async fn modify_headers(&self, headers: &mut Headers);
}

#[derive(Clone)]
pub struct StaticHeadersInterceptor {
    pub path_prefix: String,
    pub headers: HashMap<String, String>,
}

#[async_trait]
impl ProxyInterceptor for StaticHeadersInterceptor {
    fn applies(&self, uri: &str) -> bool {
        uri.starts_with(&self.path_prefix)
    }
    async fn modify_headers(&self, headers: &mut Headers) {
        let h = self.headers.clone();
        headers.remove("Host");
        headers.remove("Content-Length");
        for (name, value) in h.iter() {
            let header_value = value.parse::<HeaderValue>().unwrap_or_log();
            let header_name = name.parse::<HeaderName>().unwrap_or_log();
            headers.insert(header_name, header_value);
        }
    }
}

pub struct RequestHandler {
    pub interceptors: Vec<Box<dyn ProxyInterceptor>>,
}

async fn handle(
    uri: &str,
    headers: &mut Headers,
    handler: Arc<tokio::sync::RwLock<RequestHandler>>,
) {
    info!("Handling request, {}", &uri);
    let handler = handler.read().await;
    let interceptors_ref = &handler.interceptors;
    for interceptor in interceptors_ref.iter() {
        if interceptor.applies(uri) {
            interceptor.modify_headers(headers).await;
        }
    }
}

pub fn start_proxy_to_adress(
    local_port: u16,
    address: String,
    request_handler: Arc<tokio::sync::RwLock<RequestHandler>>,
) -> tokio::sync::oneshot::Sender<()> {
    kill_pid_on_port(local_port);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let request_filter = extract_request_data_filter();
    let app = warp::any()
        .and(request_filter)
        .and_then(
            move |uri: warp::path::FullPath,
                  params: Option<String>,
                  method: Method,
                  mut headers: Headers,
                  body: Bytes| {
                let request_handler = request_handler.clone();
                let address = address.clone();
                async move {
                    handle(uri.as_str(), &mut headers, request_handler).await;
                    proxy_to_and_forward_response(
                        address,
                        "".to_owned(),
                        uri,
                        params,
                        method,
                        headers,
                        body,
                    )
                    .await
                }
            },
        )
        .with(warp::log("proxy::log"));

    let (_addr, server) =
        warp::serve(app).bind_with_graceful_shutdown(([0, 0, 0, 0], local_port), async {
            rx.await.ok();
        });
    tokio::task::spawn(server);

    tx
}

#[cfg(target_os = "windows")]
fn kill_pid_on_port(port: u16) {
    info!("Killing {} on windows", &port);
    let process = Command::new("powershell")
        .args([
            "-Command",
            "netstat",
            "-ano",
            "|",
            "findStr",
            &format!(":{}", port),
            "|",
            "findStr",
            "LISTENING",
        ])
        .output()
        .expect("Failed to execute powershell");
    if process.status.success() {
        let res = String::from_utf8(process.stdout).expect("Failed to convert string");
        res.split("\r\n").filter(|s| !s.is_empty()).for_each(|s| {
            let pid_str = s.split_whitespace().last();
            info!("Pid? {:?} on windows", &pid_str);
            if let Some(pid_str) = pid_str {
                let pid = pid_str.parse::<u32>();
                if let Ok(pid) = pid {
                    info!("Killing PID: {}", &pid);
                    let _ = Command::new("powershell")
                        .args(["-Command", "taskkill", "/PID", &pid.to_string(), "/F"])
                        .output();
                }
            }
        });
    }
}

#[cfg(target_os = "linux")]
fn kill_pid_on_port(port: u16) {
    let mut lsof = Command::new("lsof")
        .args([format!("-i:{}", port)])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_log();
    let _ = lsof.wait();

    let mut grep_by_port = Command::new("grep")
        .arg(format!(":{}", port))
        .stdin(Stdio::from(lsof.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = grep_by_port.wait();

    let mut cut = Command::new("cut")
        .args(["-d", " ", "-f", "2"])
        .stdin(Stdio::from(grep_by_port.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = cut.wait();

    let _ = Command::new("xargs")
        .arg("kill")
        .stdin(Stdio::from(cut.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
        .wait();
}

#[cfg(target_os = "macos")]
fn kill_pid_on_port(port: u16) {
    // lsof -X -i -n | grep :62809 | cut -d' ' -f 2 | xargs kill
    let mut lsof = Command::new("lsof")
        .args(["-X", "-i", "-n"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_log();
    let _ = lsof.wait();
    let mut grep_by_port = Command::new("grep")
        .arg(format!(":{port}"))
        .stdin(Stdio::from(lsof.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_log();
    let _ = grep_by_port.wait();
    let grep_by_listen = Command::new("grep")
        .arg("(LISTEN)")
        .stdin(Stdio::from(grep_by_port.stdout.unwrap()))
        .output()
        .unwrap();
    let res = String::from_utf8(grep_by_listen.stdout).expect("Failed to convert string");
    for line in res.lines() {
        info!("lsof line: {}", &line);
        if let Some(pid_str) = trim_whitespace_v2(line).split_whitespace().nth(1) {
            if let Ok(pid) = pid_str.parse::<u32>() {
                warn!("Killing pid {pid}");
                let _ = Command::new("kill").arg(pid_str).spawn().unwrap().wait();
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn trim_whitespace_v2(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    s.split_whitespace().for_each(|w| {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(w);
    });
    result
}

#[derive(Debug)]
pub enum ProxyError {
    ErrorSsoToken,
    Unknown,
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Proxy Error: {}",
            match self {
                Self::ErrorSsoToken => "Invalid sso token",
                Self::Unknown => "Unknown",
            }
        )
    }
}
