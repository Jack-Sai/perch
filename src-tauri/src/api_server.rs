//! 本地只读 API：仅监听 127.0.0.1，提供 /status、/metrics、/profiles。
//! 端口可配置，默认关闭；可选 Bearer Token。

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::json;
use tokio::sync::oneshot;

use crate::config::{self, AppConfig};
use crate::monitor;
use crate::state::AppState;

struct ServerHandle {
    shutdown: oneshot::Sender<()>,
}

static HANDLE: OnceLock<Mutex<Option<ServerHandle>>> = OnceLock::new();
static LAST_ERROR: OnceLock<Mutex<String>> = OnceLock::new();

fn handle_slot() -> &'static Mutex<Option<ServerHandle>> {
    HANDLE.get_or_init(|| Mutex::new(None))
}

fn error_slot() -> &'static Mutex<String> {
    LAST_ERROR.get_or_init(|| Mutex::new(String::new()))
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiStatus {
    pub enabled: bool,
    pub running: bool,
    pub port: u16,
    pub error: Option<String>,
}

fn authorized(cfg: &AppConfig, headers: &HeaderMap) -> bool {
    let token = cfg.global.api_token.trim();
    if token.is_empty() {
        return true;
    }
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim() == format!("Bearer {token}"))
        .unwrap_or(false)
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "unauthorized" })),
    )
        .into_response()
}

fn load_cfg() -> AppConfig {
    config::load_config().unwrap_or_else(|_| config::default_config())
}

async fn status_handler(State(app): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cfg = load_cfg();
    if !authorized(&cfg, &headers) {
        return unauthorized();
    }
    let (status, pid, profile, log_path) = {
        let inner = app.inner.lock().unwrap();
        (
            inner.status.as_str().to_string(),
            inner.pid,
            inner.running_profile.clone(),
            inner.log_path.clone(),
        )
    };
    let error = match app.inner.lock().unwrap().status.clone() {
        crate::state::ServerStatus::Error(e) => Some(e),
        _ => None,
    };
    Json(json!({
        "app": "perch",
        "api_version": 1,
        "status": status,
        "pid": pid,
        "profile": profile,
        "error": error,
        "log_path": log_path,
        "profiles_count": cfg.profiles.len(),
    }))
    .into_response()
}

async fn metrics_handler(State(app): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cfg = load_cfg();
    if !authorized(&cfg, &headers) {
        return unauthorized();
    }
    match tauri::async_runtime::spawn_blocking(move || monitor::snapshot(&app)).await {
        Ok(metrics) => Json(metrics).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("metrics unavailable: {e}") })),
        )
            .into_response(),
    }
}

fn mask_env(line: &str) -> String {
    match line.split_once('=') {
        Some((key, value)) if !value.is_empty() => format!("{key}=***"),
        Some((key, _)) => format!("{key}="),
        None => "***".into(),
    }
}

async fn profiles_handler(State(_app): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let cfg = load_cfg();
    if !authorized(&cfg, &headers) {
        return unauthorized();
    }
    let profiles: Vec<_> = cfg
        .profiles
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "model_file": Path::new(&p.model_path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string()),
                "host": p.host,
                "port": p.port,
                "ngl": p.ngl,
                "threads": p.threads,
                "ctx_size": p.ctx_size,
                "flash_attn": p.flash_attn,
                "env": p.env_vars.iter().map(|v| mask_env(v)).collect::<Vec<_>>(),
            })
        })
        .collect();
    Json(json!({ "profiles": profiles })).into_response()
}

fn router(app: Arc<AppState>) -> Router {
    Router::new()
        .route("/status", get(status_handler))
        .route("/metrics", get(metrics_handler))
        .route("/profiles", get(profiles_handler))
        .with_state(app)
}

/// 根据当前配置启动/停止/重启只读 API 服务。
pub fn sync_server(app: Arc<AppState>) {
    if let Some(handle) = handle_slot().lock().unwrap().take() {
        let _ = handle.shutdown.send(());
    }
    error_slot().lock().unwrap().clear();

    let cfg = load_cfg();
    if !cfg.global.api_enabled {
        return;
    }
    let addr = SocketAddr::from(([127, 0, 0, 1], cfg.global.api_port));
    let listener = match std::net::TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => {
            *error_slot().lock().unwrap() = format!("{addr} 绑定失败: {e}");
            return;
        }
    };
    let _ = listener.set_nonblocking(true);

    let (tx, rx) = oneshot::channel::<()>();
    *handle_slot().lock().unwrap() = Some(ServerHandle { shutdown: tx });

    let app_for_task = Arc::clone(&app);
    tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::TcpListener::from_std(listener) {
            Ok(l) => l,
            Err(e) => {
                *error_slot().lock().unwrap() = format!("初始化本地 API 失败: {e}");
                return;
            }
        };
        let _ = axum::serve(listener, router(app_for_task))
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await;
    });
}

#[tauri::command]
pub fn api_status_cmd() -> ApiStatus {
    let cfg = load_cfg();
    let error = error_slot().lock().unwrap().clone();
    ApiStatus {
        enabled: cfg.global.api_enabled,
        running: handle_slot().lock().unwrap().is_some(),
        port: cfg.global.api_port,
        error: if error.is_empty() { None } else { Some(error) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn cfg_with_token(token: &str) -> AppConfig {
        let mut cfg = config::default_config();
        cfg.global.api_token = token.into();
        cfg
    }

    #[test]
    fn auth_allows_empty_token() {
        let cfg = cfg_with_token("");
        assert!(authorized(&cfg, &HeaderMap::new()));
    }

    #[test]
    fn auth_requires_matching_bearer() {
        let cfg = cfg_with_token("abc123");
        let mut ok = HeaderMap::new();
        ok.insert("authorization", HeaderValue::from_static("Bearer abc123"));
        assert!(authorized(&cfg, &ok));

        let mut bad = HeaderMap::new();
        bad.insert("authorization", HeaderValue::from_static("Bearer nope"));
        assert!(!authorized(&cfg, &bad));
        assert!(!authorized(&cfg, &HeaderMap::new()));
    }

    #[test]
    fn masks_env_values() {
        assert_eq!(mask_env("HF_TOKEN=secret"), "HF_TOKEN=***");
        assert_eq!(mask_env("EMPTY="), "EMPTY=");
        assert_eq!(mask_env("noequals"), "***");
    }
}
