//! 配置系统：基于 TOML 的全局设置与多套启动配置方案。
//!
//! 配置文件位于 `<USERPROFILE>\..perch\config.toml`。
//! 所有字段均带 `#[serde(default)]`，旧版配置缺失字段时回退默认值、不报错。

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const APP_DIR_NAME: &str = ".perch";
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// 全局设置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// 界面语言："zh-CN" | "en-US"
    pub language: String,
    /// 主题："light" | "dark" | "system"
    pub theme: String,
    /// 上次使用的配置名（启动自动选中）
    pub last_profile: String,
    /// 开机自启（默认关闭）
    pub autostart: bool,
    /// 关闭主窗口时隐藏到托盘（false 则直接退出）
    pub close_to_tray: bool,
    /// 退出启动器时保留 server 运行
    pub exit_keep_server: bool,
    /// 监控图表默认窗口（分钟）
    pub monitor_window_minutes: u32,
    /// 监控采样间隔（秒）
    pub monitor_sample_seconds: f32,
    /// llama-server.exe 的绝对路径（解析后写入）
    pub binary_path: String,
    /// 本地 API 是否开启（默认关闭）
    pub api_enabled: bool,
    /// 本地 API 端口
    pub api_port: u16,
    /// 本地 API token（开启时生成）
    pub api_token: String,
    /// 首次使用向导是否已完成
    pub first_run_done: bool,
    /// 进程 CPU 告警阈值（%）
    pub alert_cpu_percent: u32,
    /// 进程内存占系统内存告警阈值（%）
    pub alert_mem_percent: u32,
    /// GPU 利用率告警阈值（%）
    pub alert_gpu_util_percent: u32,
    /// GPU 温度告警阈值（摄氏度）
    pub alert_gpu_temp_c: u32,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: "system".into(),
            last_profile: "default".into(),
            autostart: false,
            close_to_tray: true,
            exit_keep_server: false,
            monitor_window_minutes: 5,
            monitor_sample_seconds: 1.0,
            binary_path: String::new(),
            api_enabled: false,
            api_port: 3999,
            api_token: String::new(),
            first_run_done: false,
            alert_cpu_percent: 90,
            alert_mem_percent: 90,
            alert_gpu_util_percent: 90,
            alert_gpu_temp_c: 85,
        }
    }
}

/// 单套启动配置方案。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// 配置名（唯一）
    pub name: String,
    /// 模型 GGUF 路径
    pub model_path: String,
    /// 绑定 host，默认 127.0.0.1
    pub host: String,
    /// 端口，默认 8080
    pub port: u16,
    /// -ngl，GPU 层数；-1 表示未设置
    pub ngl: i32,
    /// -t，线程数；0 表示未设置
    pub threads: u32,
    /// -c，上下文长度；0 表示未设置
    pub ctx_size: u32,
    /// --flash-attn
    pub flash_attn: bool,
    /// 自定义参数文本区（整段命令行）
    pub extra_params: String,
    /// 工作目录
    pub work_dir: String,
    /// 额外环境变量（每行 "KEY=VALUE"）
    pub env_vars: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".into(),
            model_path: String::new(),
            host: "127.0.0.1".into(),
            port: 8080,
            ngl: -1,
            threads: 0,
            ctx_size: 0,
            flash_attn: false,
            extra_params: String::new(),
            work_dir: String::new(),
            env_vars: Vec::new(),
        }
    }
}

/// 根配置：全局设置 + 多套方案。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub global: GlobalConfig,
    pub profiles: Vec<Profile>,
}

/// 返回配置目录 `C:\Users\<user>\.perch`。
pub fn app_dir() -> PathBuf {
    let base = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join(APP_DIR_NAME)
}

/// 返回配置文件完整路径。
pub fn config_path() -> PathBuf {
    app_dir().join(CONFIG_FILE_NAME)
}

/// 确保配置目录存在，返回其路径。
pub fn ensure_app_dir() -> Result<PathBuf, String> {
    let dir = app_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// 默认配置：一套 default 方案。
pub fn default_config() -> AppConfig {
    AppConfig {
        global: GlobalConfig::default(),
        profiles: vec![Profile::default()],
    }
}

/// 加载配置；文件不存在时创建默认配置并落盘。
/// 旧版配置缺失字段自动使用默认值。
pub fn load_config() -> Result<AppConfig, String> {
    let path = config_path();
    if !path.exists() {
        let cfg = default_config();
        save_config(&cfg)?;
        return Ok(cfg);
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let cfg: AppConfig = toml::from_str(&content).map_err(|e| e.to_string())?;
    Ok(cfg)
}

/// 保存配置到磁盘。
pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    ensure_app_dir().map_err(|e| e.to_string())?;
    let content = toml::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(), content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 按名称查找方案；不存在返回 None。
pub fn find_profile<'a>(cfg: &'a AppConfig, name: &str) -> Option<&'a Profile> {
    cfg.profiles.iter().find(|p| p.name == name)
}

// ----- Tauri 命令 -----

/// 加载并返回完整配置。
#[tauri::command]
pub fn load_config_cmd() -> Result<AppConfig, String> {
    load_config()
}

/// 整体保存配置（前端编辑后调用）。
#[tauri::command]
pub fn save_config_cmd(
    cfg: AppConfig,
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Result<(), String> {
    save_config(&cfg)?;
    crate::api_server::sync_server(std::sync::Arc::clone(&state));
    crate::tray::refresh(&app, &state);
    Ok(())
}

/// 新建一套配置方案；同名时报错。
#[tauri::command]
pub fn create_profile(
    name: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Result<AppConfig, String> {
    let mut cfg = load_config()?;
    if cfg.profiles.iter().any(|p| p.name == name) {
        return Err(format!("已存在同名配置：{name}"));
    }
    let mut p = Profile::default();
    p.name = name;
    cfg.profiles.push(p);
    save_config(&cfg)?;
    crate::tray::refresh(&app, &state);
    Ok(cfg)
}

/// 删除一套配置方案；至少保留一套。
#[tauri::command]
pub fn delete_profile(
    name: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Result<AppConfig, String> {
    let mut cfg = load_config()?;
    if cfg.profiles.len() <= 1 {
        return Err("至少保留一套配置".into());
    }
    cfg.profiles.retain(|p| p.name != name);
    if cfg.global.last_profile == name {
        cfg.global.last_profile = cfg.profiles[0].name.clone();
    }
    save_config(&cfg)?;
    crate::tray::refresh(&app, &state);
    Ok(cfg)
}

/// 记录上次使用的配置名。
#[tauri::command]
pub fn set_last_profile(
    name: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Result<AppConfig, String> {
    let mut cfg = load_config()?;
    cfg.global.last_profile = name;
    save_config(&cfg)?;
    crate::tray::refresh(&app, &state);
    Ok(cfg)
}

fn file_name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 导出配置为 TOML。mask_model=true 时将模型路径替换为 `<MODEL_PATH>`，
/// 并在文件头部注释中保留文件名提示（不泄露完整路径）。
#[tauri::command]
pub fn export_config_cmd(path: String, mask_model: bool) -> Result<(), String> {
    let cfg = load_config()?;
    let mut header = String::from("# Perch configuration export\n");
    let mut out_cfg = cfg.clone();
    if mask_model {
        for p in &mut out_cfg.profiles {
            if !p.model_path.is_empty() {
                header.push_str(&format!(
                    "# profile {}: model file = {}\n",
                    p.name,
                    file_name_of(&p.model_path)
                ));
                p.model_path = "<MODEL_PATH>".into();
            }
        }
    }
    let body = toml::to_string_pretty(&out_cfg).map_err(|e| e.to_string())?;
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(&path, format!("{header}{body}")).map_err(|e| e.to_string())?;
    Ok(())
}

/// 导入配置 TOML，合并到现有配置。同名方案自动追加后缀，避免覆盖。
/// 返回合并后的完整配置；模型路径为占位符的方案由前端提示重新指定。
#[tauri::command]
pub fn import_config_cmd(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, std::sync::Arc<crate::state::AppState>>,
) -> Result<AppConfig, String> {
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let imported: AppConfig =
        toml::from_str(&content).map_err(|e| format!("TOML 解析失败：{e}"))?;
    if imported.profiles.is_empty() {
        return Err("导入文件中没有任何配置方案".into());
    }
    let mut cfg = load_config()?;
    for mut p in imported.profiles {
        if p.name.trim().is_empty() {
            p.name = "imported".into();
        }
        let base = p.name.clone();
        let mut idx = 1;
        while cfg.profiles.iter().any(|x| x.name == p.name) {
            idx += 1;
            p.name = format!("{base}-imported{idx}");
        }
        cfg.profiles.push(p);
    }
    save_config(&cfg)?;
    crate::api_server::sync_server(std::sync::Arc::clone(&state));
    crate::tray::refresh(&app, &state);
    Ok(cfg)
}
