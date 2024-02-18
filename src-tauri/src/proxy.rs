use crate::{AsyncTaskManager, ProxyEventMessage};

use filepath::FilePath;
use log::{error, info, warn};
use port_killer::kill;
use shared_child::SharedChild;
use std::collections::HashMap;
use std::fs::{self, File};
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tauri::Window;
use tempdir::TempDir;
use tokio::time::sleep;
use tracing_unwrap::ResultExt;
use warp::http::HeaderName;
use warp::http::HeaderValue;
use warp::hyper::body::Bytes;
use warp::hyper::Method;
use warp::Filter as WarpFilter;
use warp_reverse_proxy::{extract_request_data_filter, proxy_to_and_forward_response, Headers};

pub async fn start_aws_ssm_proxy(
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

    println!(
        "Local: {}, target: {}, access: {}",
        &local_port, &target_port, &access_port
    );

    kill_pid_on_port(local_port).await;

    let tmp_dir = TempDir::new("wombat").unwrap();
    let out_log: File =
        File::create(tmp_dir.path().join(format!("out-{}.log", local_port))).unwrap();
    let err_log: File =
        File::create(tmp_dir.path().join(format!("err-{}.log", local_port))).unwrap();

    let out_log_path = format!("{}", out_log.path().unwrap().display());
    let err_log_path = format!("{}", err_log.path().unwrap().display());

    command.stdout(Stdio::from(out_log));
    command.stderr(Stdio::from(err_log));

    info!("Out log path: {:?}", out_log_path.clone());
    info!("Error log path: {:?}", err_log_path.clone());
    info!("Executnig cmd: {:?} ", command);

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

        info!("Output: {}", contents);
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
            error!("Failed to start proxy: {}", contents);
            window
                .emit(
                    "proxy-end",
                    ProxyEventMessage::new(arn.clone(), "ERROR".into(), access_port),
                )
                .unwrap_or_log();

            kill_pid_on_port(local_port).await;
            return;
        }
    }
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
                ProxyEventMessage::new(arn.clone(), "STARTED".into(), access_port),
            )
            .unwrap_or_log();
        let _ = child_arc_clone.wait();

        if let Some(handle) = abort_on_exit {
            info!("Killing dependant job");
            handle.abort();
        }
        window
            .emit(
                "proxy-end",
                ProxyEventMessage::new(arn.clone(), "END".into(), access_port),
            )
            .unwrap_or_log();
        kill_pid_on_port(local_port).await;
    });
}

#[derive(Clone)]
pub struct ProxyInterceptor {
    pub path_prefix: String,
    pub headers: HashMap<String, String>,
}

impl ProxyInterceptor {
    fn applies(&self, uri: &str) -> bool {
        return uri.starts_with(&self.path_prefix);
    }
    fn modify_headers(&self, headers: &mut Headers) {
        let h = self.headers.clone();
        for (name, value) in h.iter() {
            let header_value = value.parse::<HeaderValue>().unwrap_or_log();
            let header_name = name.parse::<HeaderName>().unwrap_or_log();
            headers.insert(header_name, header_value);
        }
    }
}

pub async fn start_proxy_to_aws_proxy(
    local_port: u16,
    aws_local_port: u16,
    proxy_intercepter: ProxyInterceptor,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        let request_filter = extract_request_data_filter();

        let app = warp::any().and(request_filter).and_then(
            move |uri: warp::path::FullPath,
                  params: Option<String>,
                  method: Method,
                  mut headers: Headers,
                  body: Bytes| {
                if proxy_intercepter.applies(uri.as_str()) {
                    proxy_intercepter.modify_headers(&mut headers);
                }

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

#[cfg(target_os = "windows")]
async fn kill_pid_on_port(port: u16) {
    info!("Killing {} on windows", &port);
    let process = Command::new("powershell")
        .args(&[
            "-Command",
            "netstat",
            "-ano",
            "|",
            "findStr",
            &format!(":{}", port),
            "|",
            "findStr",
            "LISTENING"
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
                        .args(&["-Command", "taskkill", "/PID", &pid.to_string(), "/F"])
                        .output();
                }
            }
        });
    }
}

#[cfg(target_os = "linux")]
async fn kill_pid_on_port(port: u16) {
    let _ = tokio::task::spawn(async move {
        info!("Trying to kill process on local port: {}", port);
        let kill_result = kill(port);
        match kill_result {
            Ok(res) => info!("Killed: {}", res),
            Err(err) => warn!("Killing failed, {}", err),
        }
    })
    .await;
}

#[cfg(target_os = "macos")]
async fn kill_pid_on_port(port: u16) {
    // lsof -X -i -n | grep :62809 | cut -d' ' -f 2 | xargs kill
    let lsof = Command::new("lsof")
        .args(&["-X", "-i", "-n"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_log();
    let grep_by_port = Command::new("grep")
        .arg(format!(":{}", port))
        .stdin(Stdio::from(lsof.stdout.unwrap())) // Pipe through.
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let cut = Command::new("cut")
        .args(["-d", " ", "-f", "2"])
        .stdin(Stdio::from(grep_by_port.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = Command::new("xargs")
        .arg("kill")
        .stdin(Stdio::from(cut.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
}
