//! 资源监控：基于 sysinfo 采集目标进程 CPU/内存，优先 NVML、失败时降级 nvidia-smi 采集 GPU。

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use sysinfo::{Pid, ProcessesToUpdate, System};

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
    /// 系统总内存（字节）
    pub mem_total_bytes: u64,
    /// 进程内存占系统总内存比例（%）
    pub mem_percent: f32,
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
pub struct GpuInfo {
    pub name: Option<String>,
    pub util: Option<f32>,
    pub mem_used: Option<u64>,
    pub mem_total: Option<u64>,
    pub temp: Option<f32>,
    pub source: Option<String>,
}

fn parse_f32(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok()
}

fn parse_u64(s: &str) -> Option<u64> {
    s.trim().parse::<u64>().ok()
}

fn nvml_instance() -> Option<&'static nvml_wrapper::Nvml> {
    static NVML: OnceLock<Option<nvml_wrapper::Nvml>> = OnceLock::new();
    NVML.get_or_init(|| nvml_wrapper::Nvml::init().ok())
        .as_ref()
}

/// 优先通过 NVML 查询指定索引的 GPU。
fn query_nvml(index: u32) -> Option<GpuInfo> {
    use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
    let nvml = nvml_instance()?;
    let device = nvml.device_by_index(index).ok()?;
    let name = device.name().ok();
    let util = device.utilization_rates().ok().map(|u| u.gpu as f32);
    let memory = device.memory_info().ok();
    let temp = device
        .temperature(TemperatureSensor::Gpu)
        .ok()
        .map(|t| t as f32);
    Some(GpuInfo {
        name,
        util,
        mem_used: memory.as_ref().map(|m| m.used),
        mem_total: memory.map(|m| m.total),
        temp,
        source: Some("nvml".into()),
    })
}

/// 通过 nvidia-smi 查询指定索引的 GPU。
fn query_nvidia_smi(index: u32) -> Option<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .args([
            "-i",
            &index.to_string(),
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
pub(crate) fn gpu_info(index: u32) -> GpuInfo {
    static CACHE: OnceLock<Mutex<(std::time::Instant, u32, Option<GpuInfo>)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((std::time::Instant::now(), u32::MAX, None)));
    let mut guard = cache.lock().unwrap();
    if guard.1 == index && guard.0.elapsed().as_secs_f32() < 2.0 {
        return guard.2.clone().unwrap_or_default();
    }
    let info = query_nvml(index).or_else(|| query_nvidia_smi(index));
    *guard = (std::time::Instant::now(), index, info.clone());
    info.unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuEntry {
    pub index: u32,
    pub name: String,
    pub mem_total_bytes: Option<u64>,
    pub source: String,
}

/// 枚举可用 GPU（NVML 优先）。
#[tauri::command]
pub fn list_gpus_cmd() -> Vec<GpuEntry> {
    if let Some(nvml) = nvml_instance() {
        if let Ok(count) = nvml.device_count() {
            let mut out = Vec::new();
            for index in 0..count {
                if let Ok(device) = nvml.device_by_index(index) {
                    out.push(GpuEntry {
                        index,
                        name: device.name().unwrap_or_else(|_| format!("GPU {index}")),
                        mem_total_bytes: device.memory_info().ok().map(|m| m.total),
                        source: "nvml".into(),
                    });
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .creation_flags(NO_WINDOW)
        .output();
    let Ok(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut cols = line.splitn(3, ',');
            let index = parse_u64(cols.next()?)?.try_into().ok()?;
            let name = cols.next()?.trim().to_string();
            let mem = parse_u64(cols.next()?).map(|m| m * 1024 * 1024);
            Some(GpuEntry {
                index,
                name,
                mem_total_bytes: mem,
                source: "nvidia-smi".into(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn get_metrics_cmd(state: tauri::State<'_, Arc<AppState>>) -> Metrics {
    snapshot(&state)
}

/// 采集一次指标快照（供 Tauri 命令与只读 API 复用）。
pub(crate) fn snapshot(state: &AppState) -> Metrics {
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
            mem_total_bytes: 0,
            mem_percent: 0.0,
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
    let mut mem_total_bytes = 0;
    if let Some(pid) = pid {
        let mut sys = system().lock().unwrap();
        sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        if let Some(proc) = sys.process(Pid::from_u32(pid)) {
            cpu_percent = proc.cpu_usage();
            mem_bytes = proc.memory();
        }
        mem_total_bytes = sys.total_memory();
    }
    let mem_percent = if mem_total_bytes > 0 {
        mem_bytes as f32 / mem_total_bytes as f32 * 100.0
    } else {
        0.0
    };

    let gpu_index = crate::config::load_config()
        .map(|c| c.global.gpu_index)
        .unwrap_or(0);
    let gpu = gpu_info(gpu_index);
    Metrics {
        running: true,
        pid,
        cpu_percent,
        mem_bytes,
        mem_total_bytes,
        mem_percent,
        gpu_util_percent: gpu.util,
        gpu_mem_used_bytes: gpu.mem_used,
        gpu_mem_total_bytes: gpu.mem_total,
        gpu_temp_c: gpu.temp,
        gpu_name: gpu.name,
        gpu_source: gpu.source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_enumeration_does_not_panic() {
        let _ = list_gpus_cmd();
    }
}
