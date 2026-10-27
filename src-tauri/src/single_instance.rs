//! 单实例机制：全局命名 Mutex（Global\PerchSingleInstance）确保跨完整性级别单实例；
//! 重复启动时通过命名管道 \\.\pipe\PerchFocus 请求主实例聚焦窗口。

use std::ptr;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tauri::Manager;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_PIPE_CONNECTED, FALSE, GENERIC_READ, GENERIC_WRITE,
    GetLastError, INVALID_HANDLE_VALUE, TRUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex};

use crate::config;
use crate::process;
use crate::state::{AppState, ServerStatus};

const MUTEX_NAME: &str = "Global\\PerchSingleInstance";
pub const PIPE_NAME: &str = "\\\\.\\pipe\\PerchFocus";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 是否为重复启动的二级实例（Mutex 已存在）。
pub fn is_secondary_instance() -> bool {
    let name = to_wide(MUTEX_NAME);
    let handle = unsafe { CreateMutexW(ptr::null(), FALSE, name.as_ptr()) };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        // 创建失败，保守放行
        return false;
    }
    let already = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if already {
        // 二级实例：不持有该 Mutex
        unsafe {
            let _ = ReleaseMutex(handle);
            CloseHandle(handle);
        }
    } else {
        // 主实例：不关闭句柄，Mutex 所有权保持至进程结束
        let _ = handle;
    }
    already
}

/// 二级实例向主实例发送聚焦请求。
pub fn focus_primary() {
    let _ = request_primary("{\"cmd\":\"focus\"}");
}

/// 向主实例发送一行 JSON 请求并读取一行 JSON 响应。
pub fn request_primary(request: &str) -> Result<String, String> {
    let name = to_wide(PIPE_NAME);
    let h = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        )
    };
    if h == 0 || h == INVALID_HANDLE_VALUE {
        return Err("Perch 未运行或命名管道不可用".into());
    }
    let payload = format!("{request}\n");
    let mut written: u32 = 0;
    let ok = unsafe {
        WriteFile(
            h,
            payload.as_ptr(),
            payload.len() as u32,
            &mut written,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        unsafe { CloseHandle(h) };
        return Err("写入命名管道失败".into());
    }

    let mut response = Vec::new();
    loop {
        let mut buf = [0u8; 4096];
        let mut read: u32 = 0;
        let ok = unsafe {
            ReadFile(
                h,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut read,
                ptr::null_mut(),
            )
        };
        if ok == 0 || read == 0 {
            break;
        }
        response.extend_from_slice(&buf[..read as usize]);
        if response.len() > 256 * 1024 || response.contains(&b'\n') {
            break;
        }
    }
    unsafe { CloseHandle(h) };

    if response.is_empty() {
        return Err("主实例未返回响应".into());
    }
    Ok(String::from_utf8_lossy(&response).trim().to_string())
}

fn status_value(state: &Arc<AppState>) -> Value {
    let inner = state.inner.lock().unwrap();
    let (status, error) = match &inner.status {
        ServerStatus::Error(e) => ("error", Some(e.clone())),
        other => (other.as_str(), None),
    };
    json!({
        "ok": status != "error",
        "status": status,
        "pid": inner.pid,
        "profile": inner.running_profile,
        "error": error,
    })
}

fn wait_for(state: &Arc<AppState>, want: &[&str], timeout: Duration) -> ServerStatus {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let status = process::get_status(state);
        let current = match &status {
            ServerStatus::Error(_) => "error",
            other => other.as_str(),
        };
        if want.contains(&current) || std::time::Instant::now() >= deadline {
            return status;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn handle_request(app: &tauri::AppHandle, request: &str) -> Value {
    let state = app.state::<Arc<AppState>>().inner().clone();
    let req: Value = serde_json::from_str(request).unwrap_or_else(|_| json!({ "cmd": "focus" }));
    let cmd = req["cmd"].as_str().unwrap_or("focus");
    match cmd {
        "ping" => json!({ "ok": true, "pong": true }),
        "status" => status_value(&state),
        "focus" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
            json!({ "ok": true, "action": "focus" })
        }
        "start" => {
            let cfg = match config::load_config() {
                Ok(c) => c,
                Err(e) => return json!({ "ok": false, "error": e }),
            };
            let wanted = req["profile"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or(&cfg.global.last_profile);
            let profile = cfg
                .profiles
                .iter()
                .find(|p| p.name == wanted)
                .or_else(|| cfg.profiles.first())
                .cloned();
            let Some(profile) = profile else {
                return json!({ "ok": false, "error": "没有可用的配置方案" });
            };
            let profile_name = profile.name.clone();
            if let Err(e) = process::start_server(app, &state, profile) {
                return json!({ "ok": false, "status": process::get_status(&state).as_str(), "error": e });
            }
            let status = wait_for(&state, &["running", "error"], Duration::from_secs(120));
            let mut result = status_value(&state);
            result["profile"] = json!(profile_name);
            if matches!(status, ServerStatus::Launching) {
                result["ok"] = json!(false);
                result["error"] = json!("启动超时");
            }
            result
        }
        "stop" => {
            if let Err(e) = process::stop_server(app, &state) {
                return json!({ "ok": false, "error": e });
            }
            let status = wait_for(&state, &["idle", "error"], Duration::from_secs(60));
            let mut result = status_value(&state);
            result["ok"] = json!(matches!(status, ServerStatus::Idle));
            result
        }
        other => json!({ "ok": false, "error": format!("未知命令: {other}") }),
    }
}

/// 主实例启动命名管道监听：聚焦窗口、查询状态、启动/停止服务（供 CLI 使用）。
pub fn start_focus_listener(app: tauri::AppHandle) {
    let name = to_wide(PIPE_NAME);
    std::thread::spawn(move || {
        loop {
            let pipe = unsafe {
                CreateNamedPipeW(
                    name.as_ptr(),
                    PIPE_ACCESS_DUPLEX,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    1,
                    4096,
                    4096,
                    0,
                    ptr::null(),
                )
            };
            if pipe == 0 || pipe == INVALID_HANDLE_VALUE {
                break;
            }
            let connected = unsafe { ConnectNamedPipe(pipe, ptr::null_mut()) } == TRUE
                || unsafe { GetLastError() } == ERROR_PIPE_CONNECTED;
            if connected {
                let mut buf = [0u8; 4096];
                let mut read: u32 = 0;
                let ok = unsafe {
                    ReadFile(
                        pipe,
                        buf.as_mut_ptr(),
                        buf.len() as u32,
                        &mut read,
                        ptr::null_mut(),
                    )
                };
                if ok != 0 && read > 0 {
                    let request = String::from_utf8_lossy(&buf[..read as usize])
                        .trim()
                        .to_string();
                    let response = handle_request(&app, &request);
                    let payload = format!("{response}\n");
                    let mut written: u32 = 0;
                    unsafe {
                        WriteFile(
                            pipe,
                            payload.as_ptr(),
                            payload.len() as u32,
                            &mut written,
                            ptr::null_mut(),
                        );
                    }
                }
            }
            unsafe {
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
            }
        }
    });
}
