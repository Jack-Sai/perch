//! llama-server 二进制发现、后端类型识别与 `--help` 扫描缓存。

use serde::{Deserialize, Serialize};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config;

const NO_WINDOW: u32 = 0x08000000;

/// 单条 `--help` 解析出的参数定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpArg {
    /// 长参数名（不含前导 --），如 "flash-attn"
    pub name: String,
    /// 短参数名（不含前导 -），如 "ngl"；无则为空
    pub short: String,
    /// 参数值占位，如 "N" / "PATH"；无则为空
    pub value: String,
    /// 说明文字
    pub description: String,
    /// 是否接受值
    pub takes_value: bool,
}

/// 二进制识别结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInfo {
    pub path: String,
    /// 后端类型：CUDA / Vulkan / SYCL / CPU / Unknown
    pub backend: String,
    pub version: String,
    pub help_args: Vec<HelpArg>,
}

/// `--help` 缓存文件结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HelpCache {
    exe_path: String,
    mtime: i64,
    size: u64,
    help_args: Vec<HelpArg>,
    scanned_at: i64,
}

/// 后端优先级（数值越大越优先）。
fn backend_priority(b: &str) -> u8 {
    match b {
        "CUDA" => 4,
        "Vulkan" => 3,
        "SYCL" => 2,
        "CPU" => 1,
        _ => 0,
    }
}

/// 在 PATH 中查找最靠前的 llama-server.exe。
pub fn find_in_path() -> Option<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(';') {
        let candidate = Path::new(dir).join("llama-server.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// 依据目录名与同目录动态库识别后端类型。
pub fn identify_backend(exe_path: &Path) -> String {
    let s = exe_path.to_string_lossy().to_lowercase();
    // 目录名优先
    for (kw, backend) in [
        ("cuda", "CUDA"),
        ("vulkan", "Vulkan"),
        ("sycl", "SYCL"),
        ("cpu", "CPU"),
    ] {
        if s.contains(&format!("/{kw}")) || s.contains(&format!("\\{kw}")) {
            return backend.to_string();
        }
    }
    // 同目录动态库
    if let Some(parent) = exe_path.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains("ggml-cuda") {
                    return "CUDA".to_string();
                } else if name.contains("ggml-vulkan") {
                    return "Vulkan".to_string();
                } else if name.contains("ggml-sycl") {
                    return "SYCL".to_string();
                } else if name.contains("ggml-cpu") || name.contains("ggml-base") {
                    return "CPU".to_string();
                }
            }
        }
    }
    "Unknown".to_string()
}

/// 给定输入（文件或目录），解析出 llama-server.exe 路径。
/// 目录按固定相对路径优先，命中多后端时按优先级选择。
pub fn resolve_binary(input: &str) -> Result<PathBuf, String> {
    let p = Path::new(input);
    if p.is_file() {
        if p.file_name()
            .map(|n| n == "llama-server.exe")
            .unwrap_or(false)
        {
            return Ok(p.to_path_buf());
        }
        return Err("指定的文件不是 llama-server.exe".into());
    }
    if p.is_dir() {
        // 固定相对路径优先级
        let fixed = ["cuda", "vulkan", "sycl", "cpu", "build/bin"];
        let mut candidates: Vec<(String, PathBuf)> = Vec::new();
        for rel in fixed {
            let c = p.join(rel).join("llama-server.exe");
            if c.is_file() {
                candidates.push((identify_backend(&c), c));
            }
        }
        // 有限深度递归（深度 ≤ 3）
        if candidates.is_empty() {
            collect_recursive(p, 0, 3, &mut candidates);
        }
        if candidates.is_empty() {
            return Err("目录中未找到 llama-server.exe".into());
        }
        candidates.sort_by(|a, b| backend_priority(&b.0).cmp(&backend_priority(&a.0)));
        return Ok(candidates[0].1.clone());
    }
    Err("输入的路径既不是文件也不是目录".into())
}

fn collect_recursive(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<(String, PathBuf)>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name == ".git" || name == "node_modules" || name == "target" {
                continue;
            }
            collect_recursive(&path, depth + 1, max_depth, out);
        } else if entry.file_name() == "llama-server.exe" {
            out.push((identify_backend(&path), path));
        }
    }
}

/// 获取版本号（`--version`）。
pub fn get_version(exe: &Path) -> String {
    run_capture(exe, &["--version"])
        .map(|out| first_line(&out))
        .unwrap_or_else(|_| "unknown".into())
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("unknown").trim().to_string()
}

/// 运行子进程并捕获 stdout+stderr。
fn run_capture(exe: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(exe)
        .args(args)
        .creation_flags(NO_WINDOW)
        .output()
        .map_err(|e| format!("无法运行 {}: {}", exe.display(), e))?;
    let mut s = String::from_utf8_lossy(&output.stdout).to_string();
    if s.trim().is_empty() {
        s = String::from_utf8_lossy(&output.stderr).to_string();
    }
    Ok(s)
}

/// 解析 `--help` 文本，提取参数定义。
fn parse_help(raw: &str) -> Vec<HelpArg> {
    let mut args = Vec::new();
    for line in raw.lines() {
        // 形如：  -ngl, --gpu-layers N      说明
        // 或：      --flash-attn          说明
        let trimmed = line.trim_start();
        if !trimmed.starts_with('-') {
            continue;
        }
        // 取前两个空白分隔的 token：短/长参数 + 值占位
        let mut parts = trimmed
            .splitn(3, char::is_whitespace)
            .filter(|s| !s.is_empty());
        let first = match parts.next() {
            Some(f) => f,
            None => continue,
        };
        let value = parts.next().unwrap_or("").to_string();
        let description = parts.next().unwrap_or("").to_string();

        // 拆分短/长
        let (short, long) = if first.contains(',') {
            let mut it = first.split(',');
            let s = it.next().unwrap_or("").trim_start_matches('-').to_string();
            let l = it.next().unwrap_or("").trim_start_matches('-').to_string();
            (s, l)
        } else if first.starts_with("--") {
            ("".into(), first.trim_start_matches('-').to_string())
        } else {
            (first.trim_start_matches('-').to_string(), "".into())
        };
        let name = if long.is_empty() {
            short.clone()
        } else {
            long.clone()
        };
        if name.is_empty() {
            continue;
        }
        let takes_value = !value.is_empty() && !value.starts_with('-');
        args.push(HelpArg {
            name,
            short,
            value,
            description,
            takes_value,
        });
    }
    args
}

fn help_cache_path() -> PathBuf {
    config::app_dir().join("help_cache.json")
}

/// 扫描 `--help` 并缓存；force 时跳过缓存校验直接重扫。
pub fn scan_help(exe: &Path, force: bool) -> Result<Vec<HelpArg>, String> {
    let meta = fs::metadata(exe).map_err(|e| e.to_string())?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let size = meta.len();

    if !force {
        if let Ok(cache_str) = fs::read_to_string(help_cache_path()) {
            if let Ok(cache) = serde_json::from_str::<HelpCache>(&cache_str) {
                if cache.exe_path == exe.to_string_lossy()
                    && cache.mtime == mtime
                    && cache.size == size
                {
                    return Ok(cache.help_args);
                }
            }
        }
    }

    let raw = run_capture(exe, &["--help"])?;
    let args = parse_help(&raw);
    let cache = HelpCache {
        exe_path: exe.to_string_lossy().to_string(),
        mtime,
        size,
        help_args: args.clone(),
        scanned_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    };
    if let Ok(dir) = config::ensure_app_dir() {
        let _ = fs::write(
            dir.join("help_cache.json"),
            serde_json::to_string_pretty(&cache).unwrap(),
        );
    }
    Ok(args)
}

/// 检测 PATH 中的二进制；未找到报错交由向导处理。
#[tauri::command]
pub fn detect_binary_cmd() -> Result<BinaryInfo, String> {
    let exe = find_in_path().ok_or("未在 PATH 中找到 llama-server.exe，请通过向导手动指定")?;
    resolve_and_build(&exe)
}

/// 解析给定路径的二进制，识别后端、版本，扫描 --help，并持久化到全局配置。
#[tauri::command]
pub fn resolve_binary_cmd(input: String) -> Result<BinaryInfo, String> {
    let exe = resolve_binary(&input)?;
    let info = resolve_and_build(&exe)?;
    // 持久化 binary_path 到全局配置
    let mut cfg = config::load_config()?;
    cfg.global.binary_path = info.path.clone();
    config::save_config(&cfg)?;
    Ok(info)
}

/// 读取当前配置中已保存的二进制信息（若未配置返回 None）。
#[tauri::command]
pub fn inspect_binary_cmd() -> Result<Option<BinaryInfo>, String> {
    let cfg = config::load_config()?;
    if cfg.global.binary_path.is_empty() {
        return Ok(None);
    }
    let exe = Path::new(&cfg.global.binary_path);
    if !exe.is_file() {
        return Err(format!("llama-server 不存在：{}", cfg.global.binary_path));
    }
    Ok(Some(resolve_and_build(exe)?))
}

/// 重新扫描当前二进制的 --help；force=true 时跳过缓存。
#[tauri::command]
pub fn rescan_help_cmd(force: bool) -> Result<BinaryInfo, String> {
    let cfg = config::load_config()?;
    if cfg.global.binary_path.is_empty() {
        return Err("尚未指定 llama-server 路径".into());
    }
    let exe = Path::new(&cfg.global.binary_path);
    if !exe.is_file() {
        return Err(format!("llama-server 不存在：{}", cfg.global.binary_path));
    }
    let backend = identify_backend(exe);
    let version = get_version(exe);
    let help_args = scan_help(exe, force).unwrap_or_default();
    Ok(BinaryInfo {
        path: exe.to_string_lossy().to_string(),
        backend,
        version,
        help_args,
    })
}

fn resolve_and_build(exe: &Path) -> Result<BinaryInfo, String> {
    let backend = identify_backend(exe);
    let version = get_version(exe);
    let help_args = scan_help(exe, false).unwrap_or_default();
    Ok(BinaryInfo {
        path: exe.to_string_lossy().to_string(),
        backend,
        version,
        help_args,
    })
}
