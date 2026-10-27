//! 命令行接口：`perch --cli status|start|stop|export`。
//! 通过命名管道与正在运行的 GUI 主实例通信。

use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

use crate::single_instance::request_primary;

const USAGE: &str = "\
Perch CLI

用法:
  perch --cli status
  perch --cli start [--profile <名称>]
  perch --cli stop
  perch --cli export <目录>
  perch --cli help

说明:
  status/start/stop 需要 Perch GUI 正在运行（start 会自动尝试启动 GUI）。
  export 可离线执行，导出配置、帮助缓存与日志。";

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
    );
}

fn connect_retry(attempts: u32) -> Result<(), String> {
    let mut last = String::new();
    for i in 0..attempts {
        match request_primary("{\"cmd\":\"ping\"}") {
            Ok(_) => return Ok(()),
            Err(e) => last = e,
        }
        if i + 1 < attempts {
            std::thread::sleep(Duration::from_millis(400));
        }
    }
    Err(last)
}

fn send(cmd: Value, attempts: u32) -> Result<Value, String> {
    connect_retry(attempts)?;
    let payload = serde_json::to_string(&cmd).map_err(|e| e.to_string())?;
    let response = request_primary(&payload)?;
    serde_json::from_str(&response).map_err(|e| format!("无法解析主实例响应: {e}"))
}

fn spawn_gui() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("无法定位可执行文件: {e}"))?;
    std::process::Command::new(exe)
        .spawn()
        .map_err(|e| format!("启动 Perch 失败: {e}"))?;
    Ok(())
}

fn do_start(profile: Option<String>) -> Value {
    let mut cmd = json!({ "cmd": "start" });
    if let Some(p) = &profile {
        cmd["profile"] = json!(p);
    }
    match send(cmd.clone(), 15) {
        Ok(v) => v,
        Err(_) => {
            if let Err(e) = spawn_gui() {
                return json!({ "ok": false, "error": e });
            }
            match send(cmd, 40) {
                Ok(v) => v,
                Err(e) => json!({ "ok": false, "error": e }),
            }
        }
    }
}

fn export_dir(dir: &str) -> Value {
    let dir = PathBuf::from(dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return json!({ "ok": false, "error": format!("创建目录失败: {e}") });
    }
    let cfg = match crate::config::load_config() {
        Ok(c) => c,
        Err(e) => return json!({ "ok": false, "error": e }),
    };
    let toml_text = match toml::to_string_pretty(&cfg) {
        Ok(t) => t,
        Err(e) => return json!({ "ok": false, "error": format!("序列化配置失败: {e}") }),
    };
    let config_out = dir.join("perch-config.toml");
    if let Err(e) = std::fs::write(&config_out, toml_text) {
        return json!({ "ok": false, "error": format!("写入配置失败: {e}") });
    }

    let app_dir = crate::config::app_dir();
    let help_src = app_dir.join("help_cache.json");
    let mut help_exported = false;
    if help_src.is_file() {
        help_exported = std::fs::copy(&help_src, dir.join("help_cache.json")).is_ok();
    }

    let mut logs = 0usize;
    let logs_src = app_dir.join("logs");
    let logs_out = dir.join("logs");
    if logs_src.is_dir() && std::fs::create_dir_all(&logs_out).is_ok() {
        if let Ok(entries) = std::fs::read_dir(&logs_src) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("log") {
                    if let Some(name) = path.file_name() {
                        if std::fs::copy(&path, logs_out.join(name)).is_ok() {
                            logs += 1;
                        }
                    }
                }
            }
        }
    }

    json!({
        "ok": true,
        "dir": dir.display().to_string(),
        "config": config_out.display().to_string(),
        "help_cache": help_exported,
        "logs": logs,
    })
}

fn exit_with(value: Value, success: bool) -> ! {
    print_json(&value);
    std::process::exit(if success { 0 } else { 1 });
}

/// 若参数中包含 `--cli`，执行 CLI 并退出；返回 false 表示应启动 GUI。
pub fn try_run(args: &[String]) -> bool {
    let Some(pos) = args.iter().position(|a| a == "--cli") else {
        return false;
    };
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
    }

    let rest = &args[pos + 1..];
    let mut cmd: Option<String> = None;
    let mut profile: Option<String> = None;
    let mut export_arg: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--profile" | "-p" => {
                profile = rest.get(i + 1).cloned();
                i += 2;
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other if other.starts_with('-') => {
                eprintln!("未知选项: {other}\n\n{USAGE}");
                std::process::exit(1);
            }
            other => {
                if cmd.is_none() {
                    cmd = Some(other.to_string());
                } else {
                    export_arg = Some(other.to_string());
                }
                i += 1;
            }
        }
    }

    match cmd.as_deref() {
        Some("status") => match send(json!({ "cmd": "status" }), 15) {
            Ok(v) => {
                let ok = v["ok"].as_bool().unwrap_or(false);
                exit_with(v, ok);
            }
            Err(e) => exit_with(json!({ "ok": false, "error": e }), false),
        },
        Some("start") => {
            let v = do_start(profile);
            let ok = v["ok"].as_bool().unwrap_or(false);
            exit_with(v, ok);
        }
        Some("stop") => match send(json!({ "cmd": "stop" }), 15) {
            Ok(v) => {
                let ok = v["ok"].as_bool().unwrap_or(false);
                exit_with(v, ok);
            }
            Err(e) => exit_with(json!({ "ok": false, "error": e }), false),
        },
        Some("export") => {
            let Some(dir) = export_arg.as_deref() else {
                eprintln!("export 需要目标目录\n\n{USAGE}");
                std::process::exit(1);
            };
            let v = export_dir(dir);
            let ok = v["ok"].as_bool().unwrap_or(false);
            exit_with(v, ok);
        }
        Some("help") | None => {
            println!("{USAGE}");
            std::process::exit(if cmd.is_none() { 1 } else { 0 });
        }
        Some(other) => {
            eprintln!("未知命令: {other}\n\n{USAGE}");
            std::process::exit(1);
        }
    }
}
