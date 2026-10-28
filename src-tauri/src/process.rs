//! 进程生命周期管理：CREATE_NO_WINDOW 启动、优雅终止与超时强杀、
//! 启动成功判定（进程存活 + 端口可连 + /health 200）、状态机与日志。

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::windows::process::CommandExt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_notification::NotificationExt;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::config::{self, Profile, load_config};
use crate::params::build_args;
use crate::state::{AppState, ServerStatus, TrayVisual};

const NO_WINDOW: u32 = 0x08000000;
const LAUNCH_TIMEOUT_SECS: u64 = 60;
const STOP_GRACE_SECS: u64 = 30;
const HEALTH_INTERVAL_MS: u64 = 500;

/// 状态变更事件（前端与托盘均订阅）。
#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub status: String,
    pub visual: String,
    pub pid: Option<u32>,
    pub running_profile: Option<String>,
}

/// 由状态推导托盘视觉。
pub fn visual_for(status: &ServerStatus) -> TrayVisual {
    match status {
        ServerStatus::Running => TrayVisual::Green,
        ServerStatus::Error(_) => TrayVisual::Red,
        _ => TrayVisual::Gray,
    }
}

/// 发送系统通知（失败时静默）。
fn notify(app: &tauri::AppHandle, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Perch")
        .body(body)
        .show();
}

/// 启动成功后进入运行期守护，检测运行中崩溃。
fn spawn_runtime_watcher(app: tauri::AppHandle, state: Arc<AppState>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(1000));
            let mut inner = state.inner.lock().unwrap();
            if !matches!(inner.status, ServerStatus::Running) {
                return;
            }
            if let Some(child) = inner.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let code = status
                            .code()
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "?".into());
                        let msg = format!("运行中意外退出（退出码 {code}）");
                        inner.status = ServerStatus::Error(msg.clone());
                        inner.child = None;
                        inner.pid = None;
                        drop(inner);
                        notify(&app, &msg);
                        emit_status(&app, &state);
                        return;
                    }
                    Ok(None) => {}
                    Err(_) => {}
                }
            } else {
                return;
            }
        }
    });
}

fn emit_status(app: &tauri::AppHandle, state: &AppState) {
    let inner = state.inner.lock().unwrap();
    let ev = StatusEvent {
        status: inner.status.as_str().to_string(),
        visual: match visual_for(&inner.status) {
            TrayVisual::Green => "green".into(),
            TrayVisual::Red => "red".into(),
            TrayVisual::Gray => "gray".into(),
        },
        pid: inner.pid,
        running_profile: inner.running_profile.clone(),
    };
    drop(inner);
    let _ = app.emit("perch-status", ev);
}

/// TCP 连 127.0.0.1:port 并请求 /health，期望 200。
fn check_health(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    let mut stream = match TcpStream::connect_timeout(
        &addr
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:8080".parse().unwrap()),
        Duration::from_secs(2),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    let req = "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    let resp = String::from_utf8_lossy(&buf[..n]);
    resp.contains("200")
}

/// 启动服务。要求当前状态为 Idle 或 Error。
pub fn start_server(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    profile: Profile,
) -> Result<ServerStatus, String> {
    {
        let inner = state.inner.lock().unwrap();
        if matches!(
            inner.status,
            ServerStatus::Running | ServerStatus::Launching
        ) {
            return Err("服务已在运行或启动中".into());
        }
        if matches!(inner.status, ServerStatus::Stopping) {
            return Err("正在停止，请稍候".into());
        }
    }

    let cfg = load_config()?;
    if cfg.global.binary_path.is_empty() {
        return Err("未配置 llama-server 路径，请先指定二进制".into());
    }
    let exe = std::path::Path::new(&cfg.global.binary_path);
    if !exe.is_file() {
        return Err(format!("llama-server 不存在：{}", cfg.global.binary_path));
    }

    let (args, _warnings, _overridden, _duplicated) = build_args(&profile);

    // 日志文件（启动前清理过期会话日志）
    cleanup_logs(cfg.global.log_retention_days);
    let logs_dir = config::app_dir().join("logs");
    fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let log_path = logs_dir.join(format!("server_{ts}.log"));
    let file = fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let err_file = file.try_clone().map_err(|e| e.to_string())?;

    let mut cmd = std::process::Command::new(exe);
    cmd.args(&args)
        .creation_flags(NO_WINDOW)
        .stdout(file)
        .stderr(err_file);

    let work_dir = crate::params::expand_env_vars(profile.work_dir.trim());
    if !work_dir.is_empty() {
        cmd.current_dir(&work_dir);
    }
    for entry in &profile.env_vars {
        let entry = entry.trim();
        if entry.is_empty() || entry.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = entry.split_once('=') {
            let key = key.trim();
            if !key.is_empty() {
                cmd.env(key, crate::params::expand_env_vars(value.trim()));
            }
        }
    }
    let child = cmd.spawn().map_err(|e| format!("启动失败：{e}"))?;
    let pid = child.id();

    {
        let mut inner = state.inner.lock().unwrap();
        inner.status = ServerStatus::Launching;
        inner.child = Some(child);
        inner.pid = Some(pid);
        inner.running_profile = Some(profile.name.clone());
        inner.log_path = Some(log_path.to_string_lossy().to_string());
    }
    emit_status(app, state);

    // 后台健康探测
    let app2 = app.clone();
    let state2 = state.clone();
    let port = profile.port;
    std::thread::spawn(move || {
        let start = SystemTime::now();
        loop {
            // 子进程自行退出
            {
                let mut inner = state2.inner.lock().unwrap();
                if let Some(child) = inner.child.as_mut() {
                    if let Ok(Some(status)) = child.try_wait() {
                        let code = status
                            .code()
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "?".into());
                        let msg = format!("启动失败：进程退出（退出码 {code}）");
                        inner.status = ServerStatus::Error(msg.clone());
                        inner.child = None;
                        inner.pid = None;
                        drop(inner);
                        notify(&app2, &msg);
                        emit_status(&app2, &state2);
                        return;
                    }
                }
            }
            if check_health(port) {
                {
                    let mut inner = state2.inner.lock().unwrap();
                    inner.status = ServerStatus::Running;
                }
                emit_status(&app2, &state2);
                spawn_runtime_watcher(app2.clone(), state2.clone());
                return;
            }
            if start.elapsed().unwrap_or_default().as_secs() >= LAUNCH_TIMEOUT_SECS {
                let msg = format!("启动超时（>{LAUNCH_TIMEOUT_SECS}s）");
                {
                    let mut inner = state2.inner.lock().unwrap();
                    inner.status = ServerStatus::Error(msg.clone());
                    if let Some(mut c) = inner.child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    inner.pid = None;
                }
                notify(&app2, &msg);
                emit_status(&app2, &state2);
                return;
            }
            std::thread::sleep(Duration::from_millis(HEALTH_INTERVAL_MS));
        }
    });

    Ok(ServerStatus::Launching)
}

/// 判断进程是否仍然存活（不依赖子进程句柄）。
fn pid_alive(pid: u32) -> bool {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == 259
    }
}

/// 接管已在端口上运行的 llama-server（例如由脚本或其他启动器启动）。
pub fn adopt_server(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    profile: Profile,
) -> Result<ServerStatus, String> {
    {
        let inner = state.inner.lock().unwrap();
        match inner.status {
            ServerStatus::Idle | ServerStatus::Error(_) => {}
            _ => return Err("Perch 已在管理一个运行实例".into()),
        }
    }
    let pid = crate::port::pid_for_port(profile.port)
        .ok_or_else(|| format!("端口 {} 没有监听进程", profile.port))?;
    let name = crate::port::process_name(pid).unwrap_or_default();
    if !name.to_ascii_lowercase().contains("llama") {
        return Err(format!(
            "端口 {} 被 {name} (PID {pid}) 占用，不是 llama-server，拒绝接管",
            profile.port
        ));
    }
    if !check_health(profile.port) {
        return Err(format!("PID {pid} 未通过 /health 健康检查，无法接管"));
    }
    {
        let mut inner = state.inner.lock().unwrap();
        inner.status = ServerStatus::Running;
        inner.child = None;
        inner.pid = Some(pid);
        inner.running_profile = Some(profile.name);
        inner.log_path = None;
    }
    emit_status(app, state);
    Ok(ServerStatus::Running)
}

/// 停止服务：先优雅等待，超时后强制 kill。
pub fn stop_server(app: &tauri::AppHandle, state: &Arc<AppState>) -> Result<ServerStatus, String> {
    {
        let mut inner = state.inner.lock().unwrap();
        match inner.status {
            ServerStatus::Idle | ServerStatus::Error(_) => return Ok(inner.status.clone()),
            _ => {}
        }
        inner.status = ServerStatus::Stopping;
    }
    emit_status(app, state);

    let app2 = app.clone();
    let state2 = state.clone();
    std::thread::spawn(move || {
        let start = SystemTime::now();
        // 先尝试优雅终止（taskkill 不带 /F），超时后强制 kill
        let pid = state2.inner.lock().unwrap().pid;
        if let Some(pid) = pid {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T"])
                .creation_flags(NO_WINDOW)
                .output();
        }
        loop {
            let exited = {
                let mut inner = state2.inner.lock().unwrap();
                if let Some(child) = inner.child.as_mut() {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            inner.status = ServerStatus::Idle;
                            inner.child = None;
                            inner.pid = None;
                            inner.running_profile = None;
                            inner.log_path = None;
                            true
                        }
                        _ => false,
                    }
                } else if pid.map(pid_alive).unwrap_or(false) {
                    false
                } else {
                    inner.status = ServerStatus::Idle;
                    inner.pid = None;
                    inner.running_profile = None;
                    inner.log_path = None;
                    true
                }
            };
            if exited {
                emit_status(&app2, &state2);
                return;
            }
            if start.elapsed().unwrap_or_default().as_secs() >= STOP_GRACE_SECS {
                let mut inner = state2.inner.lock().unwrap();
                if let Some(mut c) = inner.child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                inner.status = ServerStatus::Idle;
                inner.pid = None;
                inner.running_profile = None;
                inner.log_path = None;
                drop(inner);
                emit_status(&app2, &state2);
                return;
            }
            std::thread::sleep(Duration::from_millis(HEALTH_INTERVAL_MS));
        }
    });

    Ok(ServerStatus::Stopping)
}

/// 退出前同步关闭 server：先请求终止，最多等待 grace，然后强制 kill。
pub fn shutdown_blocking(state: &Arc<AppState>, grace: Duration) {
    let (pid, mut child) = {
        let mut inner = state.inner.lock().unwrap();
        (inner.pid, inner.child.take())
    };
    if child.is_none() && pid.is_none() {
        return;
    }
    if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T"])
            .creation_flags(NO_WINDOW)
            .output();
    }
    let start = SystemTime::now();
    while start.elapsed().unwrap_or_default() < grace {
        if let Some(c) = child.as_mut() {
            if matches!(c.try_wait(), Ok(Some(_))) {
                return;
            }
        } else if let Some(pid) = pid {
            if !pid_alive(pid) {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    if let Some(mut c) = child.take() {
        let _ = c.kill();
        let _ = c.wait();
    } else if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(NO_WINDOW)
            .output();
    }
}

/// 读取当前状态。
pub fn get_status(state: &Arc<AppState>) -> ServerStatus {
    state.inner.lock().unwrap().status.clone()
}

// ----- Tauri 命令 -----

#[tauri::command]
pub fn start_server_cmd(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    profile: Profile,
) -> Result<ServerStatus, String> {
    start_server(&app, state.inner(), profile)
}

#[tauri::command]
pub fn adopt_server_cmd(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    profile: Profile,
) -> Result<ServerStatus, String> {
    adopt_server(&app, state.inner(), profile)
}

#[tauri::command]
pub fn stop_server_cmd(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ServerStatus, String> {
    stop_server(&app, state.inner())
}

#[tauri::command]
pub fn get_server_status_cmd(state: tauri::State<'_, Arc<AppState>>) -> ServerStatus {
    get_status(state.inner())
}

/// 读取当前运行实例日志末尾 N 行（用于启动失败诊断）。
#[tauri::command]
pub fn read_server_log_cmd(
    state: tauri::State<'_, Arc<AppState>>,
    tail: usize,
) -> Result<Vec<String>, String> {
    let path = {
        let inner = state.inner.lock().unwrap();
        inner.log_path.clone()
    };
    let Some(path) = path else {
        return Ok(vec![]);
    };
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let content = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let n = tail.max(1).min(lines.len());
    Ok(lines
        .into_iter()
        .rev()
        .take(n)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

/// 删除超过保留期的会话日志；days=0 表示永久保留。
pub fn cleanup_logs(days: u32) {
    if days == 0 {
        return;
    }
    let dir = config::app_dir().join("logs");
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    let max_age = std::time::Duration::from_secs(days as u64 * 24 * 3600);
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("log") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if now.duration_since(modified).unwrap_or_default() > max_age {
            let _ = fs::remove_file(path);
        }
    }
}

/// 将当前会话日志导出到指定路径。
#[tauri::command]
pub fn export_server_log_cmd(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let log_path = {
        let inner = state.inner.lock().unwrap();
        inner.log_path.clone()
    };
    let Some(log_path) = log_path else {
        return Err("当前没有日志文件".into());
    };
    let bytes = fs::read(&log_path).map_err(|e| format!("读取日志失败: {e}"))?;
    fs::write(&path, bytes).map_err(|e| format!("写入日志失败: {e}"))?;
    Ok(())
}
