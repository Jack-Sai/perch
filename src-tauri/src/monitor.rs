//! 资源监控：基于 sysinfo 采集目标进程 CPU/内存，优先 NVML、失败时降级 nvidia-smi 采集 GPU。

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use sysinfo::{Pid, System, ProcessesToUpdate};

use crate::state::{AppState, ServerStatus};

const NO_WINDOW: u32 = 0x08000000;

/// 监控指标快照。
#[derive(Debug, Clone, Serialize)]
pub struct Metrics {
    pub running: bool,
    pub pid: Option<u32>,
    /// 进程 CPU 使用率（%）
    pub cpu_percent: f32,
    /// 进程内存工作集（字节）
    pub mem_bytes: u64,
    pub gpu_util_percent: Option<f32>,
    pub gpu_mem_used_bytes: Option<u64>,
    pub gpu_mem_total_bytes: Option<u64>,
    pub gpu_temp_c: Option<f32>,
    pub gpu_name: Option<String>,
    /// "nvml" | "nvidia-smi" | null
    pub gpu_source: Option<String>,
}

fn system() -> &'static Mutex<System> {
    static SYS: OnceLock<Mutex<System>> = OnceLock::new();
    SYS.get_or_init(|| Mutex::new(System::new_all()))
}

#[derive(Debug, Clone, Default)]
struct GpuInfo {
    name: Option<String>,
    util: Option<f32>,
    mem_used: Option<u64>,
    mem_total: Option<u64>,
    temp: Option<f32>,
    source: Option<String>,
}

fn parse_f32(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok()
}

fn parse_u64(s: &str) -> Option<u64> {
    s.trim().parse::<u64>().ok()
}

/// 通过 nvidia-smi 查询第一块 GPU。
fn query_nvidia_smi() -> Option<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .creation_flags(NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().find(|l| !l.trim().is_empty())?;
    let cols: Vec<&str> = line.split(',').collect();
    if cols.len() < 5 {
        return None;
    }
    Some(GpuInfo {
        name: Some(cols[0].trim().to_string()),
        util: parse_f32(cols[1]),
        mem_used: parse_u64(cols[2]).map(|m| m * 1024 * 1024),
        mem_total: parse_u64(cols[3]).map(|m| m * 1024 * 1024),
        temp: parse_f32(cols[4]),
        source: Some("nvidia-smi".into()),
    })
}

/// 采集 GPU 信息，带 2 秒缓存以降低 nvidia-smi 调用开销。
fn gpu_info() -> GpuInfo {
    static CACHE: OnceLock<Mutex<(std::time::Instant, Option<GpuInfo>)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((std::time::Instant::now(), None)));
    let mut guard = cache.lock().unwrap();
    if guard.0.elapsed().as_secs_f32() < 2.0 {
        return guard.1.clone().unwrap_or_default();
    }
    let info = query_nvidia_smi();
    *guard = (std::time::Instant::now(), info.clone());
    info.unwrap_or_default()
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
            gpu_util_percent: None,
            gpu_mem_used_bytes: None,
            gpu_mem_total_bytes: None,
            gpu_temp_c: None,
            gpu_name: None,
            gpu_source: None,
        };
    }

    let mut cpu_percent = 0.0;
    let mut mem_bytes = 0;
    if let Some(pid) = pid {
        let mut sys = system().lock().unwrap();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        if let Some(proc) = sys.process(Pid::from_u32(pid)) {
            cpu_percent = proc.cpu_usage();
            mem_bytes = proc.memory();
        }
    }

    let gpu = gpu_info();
    Metrics {
        running: true,
        pid,
        cpu_percent,
        mem_bytes,
        gpu_util_percent: gpu.util,
        gpu_mem_used_bytes: gpu.mem_used,
        gpu_mem_total_bytes: gpu.mem_total,
        gpu_temp_c: gpu.temp,
        gpu_name: gpu.name,
        gpu_source: gpu.source,
    }
}
