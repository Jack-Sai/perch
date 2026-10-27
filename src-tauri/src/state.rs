//! 全局可变状态：服务生命周期状态机、子进程句柄、运行配置与托盘句柄。

use std::process::Child;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// 生命周期状态机：Idle -> Launching -> Running -> Stopping -> Idle
/// Error 为启动失败 / 运行中崩溃的终态（需手动重置）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServerStatus {
    Idle,
    Launching,
    Running,
    Stopping,
    /// 携带错误原因
    Error(String),
}

impl ServerStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ServerStatus::Idle => "idle",
            ServerStatus::Launching => "launching",
            ServerStatus::Running => "running",
            ServerStatus::Stopping => "stopping",
            ServerStatus::Error(_) => "error",
        }
    }
}

/// 内部可变状态。
pub struct InnerState {
    pub status: ServerStatus,
    pub child: Option<Child>,
    pub pid: Option<u32>,
    pub running_profile: Option<String>,
    /// 当前运行实例的日志文件路径（stdout+stderr 合并）
    pub log_path: Option<String>,
}

/// 托盘图标状态键，用于切换图标颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayVisual {
    Gray,
    Green,
    Red,
}

/// 应用级共享状态（以 Arc 形式托管，便于在后台线程中持有）。
pub struct AppState {
    pub inner: Mutex<InnerState>,
    pub tray: Mutex<Option<tauri::tray::TrayIcon>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InnerState {
                status: ServerStatus::Idle,
                child: None,
                pid: None,
                running_profile: None,
                log_path: None,
            }),
            tray: Mutex::new(None),
        }
    }
}
