//! 端口工具：可用性检测、从指定端口向上探测空闲端口、占用进程识别。

use std::net::TcpListener;
use std::os::windows::process::CommandExt;
use std::process::Command;

use serde::Serialize;
use sysinfo::{Pid, System};

const NO_WINDOW: u32 = 0x08000000;

/// 端口状态。
#[derive(Debug, Clone, Serialize)]
pub struct PortStatus {
    pub port: u16,
    pub available: bool,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

/// 尝试监听 127.0.0.1:port 判断是否可用。
pub fn is_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// 从 start 起向上探测（1024–65535），返回首个可用端口。
pub fn find_free_port(start: u16) -> Option<u16> {
    let start = start.max(1024);
    (start..=65535).find(|p| is_available(*p))
}

/// 通过 netstat -ano 查找监听指定端口的 PID。
pub(crate) fn pid_for_port(port: u16) -> Option<u32> {
    let output = Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .creation_flags(NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let needle = format!(":{port}");
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 5 && cols[1].ends_with(&needle) && cols[3] == "LISTENING" {
            if let Ok(pid) = cols[4].parse::<u32>() {
                return Some(pid);
            }
        }
    }
    None
}

pub(crate) fn process_name(pid: u32) -> Option<String> {
    let mut sys = System::new();
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
    );
    sys.process(Pid::from_u32(pid))
        .map(|p| p.name().to_string_lossy().to_string())
}

/// 检测端口：可用则直接返回；不可用则带上占用进程信息。
#[tauri::command]
pub fn check_port_cmd(port: u16) -> PortStatus {
    if is_available(port) {
        return PortStatus {
            port,
            available: true,
            pid: None,
            process_name: None,
        };
    }
    let pid = pid_for_port(port);
    PortStatus {
        port,
        available: false,
        pid,
        process_name: pid.and_then(process_name),
    }
}

/// 从指定端口向上探测可用端口；未找到返回错误。
#[tauri::command]
pub fn find_free_port_cmd(start: u16) -> Result<u16, String> {
    find_free_port(start).ok_or_else(|| "未找到可用端口".into())
}
