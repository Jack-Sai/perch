//! 基础进程监控：基于 sysinfo 采集目标进程 CPU 使用率与内存工作集。

use std::sync::Arc;

use serde::Serialize;
use sysinfo::{Pid, System};

use crate::state::{AppState, ServerStatus};

/// 监控指标快照。
#[derive(Debug, Clone, Serialize)]
pub struct Metrics {
    pub running: bool,
    pub pid: Option<u32>,
    /// 进程 CPU 使用率（%）
    pub cpu_percent: f32,
    /// 进程内存占用（字节）
    pub mem_bytes: u64,
}

#[tauri::command]
pub fn get_metrics_cmd(state: tauri::State<'_, Arc<AppState>>) -> Metrics {
    let (pid, running) = {
        let inner = state.inner.lock().unwrap();
        (inner.pid, matches!(inner.status, ServerStatus::Running))
    };
    if !running {
        return Metrics {
            running: false,
            pid: None,
            cpu_percent: 0.0,
            mem_bytes: 0,
        };
    }
    let Some(pid) = pid else {
        return Metrics {
            running: true,
            pid: None,
            cpu_percent: 0.0,
            mem_bytes: 0,
        };
    };

    let mut sys = System::new_all();
    sys.refresh_all();
    let p = Pid::from_u32(pid);
    match sys.process(p) {
        Some(proc) => Metrics {
            running: true,
            pid: Some(pid),
            cpu_percent: proc.cpu_usage(),
            mem_bytes: proc.memory(),
        },
        None => Metrics {
            running: true,
            pid: Some(pid),
            cpu_percent: 0.0,
            mem_bytes: 0,
        },
    }
}
