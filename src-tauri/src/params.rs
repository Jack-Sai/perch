//! 参数系统：从配置方案构建 llama-server 命令行、命令预览与基础校验，
//! 以及导出可执行的 .bat 脚本。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::{Profile, load_config};

/// 命令预览与校验结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPreview {
    /// 实际执行的参数（不含可执行文件本身）
    pub args: Vec<String>,
    /// 可供展示/导出的完整命令行（含可执行文件路径）
    pub preview: String,
    /// 警告信息（非阻断）
    pub warnings: Vec<String>,
    /// 被自定义区覆盖的常用参数
    pub overridden: Vec<String>,
    /// 自定义区内重复的参数为最后一个值
    pub duplicated: Vec<String>,
}

/// 将自定义参数文本按空白切分，支持单/双引号包裹。
pub fn split_args(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else {
                    cur.push(ch);
                }
            }
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                c if c.is_whitespace() => {
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                }
                c => cur.push(c),
            },
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// 是否为参数 token（以 - 开头，且第二个字符为字母，排除负数）。
fn is_flag(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next() == Some('-') && chars.next().map(|c| c.is_alphabetic()).unwrap_or(false)
}

/// 提取 token 的原始名称（长参数或短参数，去掉前导 - 与 =值）。
fn raw_name(token: &str) -> Option<String> {
    if !is_flag(token) {
        return None;
    }
    let trimmed = token.trim_start_matches('-');
    let name = trimmed.split('=').next().unwrap_or(trimmed);
    if name.is_empty() {
        None
    } else {
        Some(name.to_lowercase())
    }
}

/// 将长短参数名归一到同一组（用于覆盖与重复判定）。
fn group_name(token: &str) -> Option<&'static str> {
    match raw_name(token)?.as_str() {
        "m" | "model" => Some("model"),
        "host" | "hostname" => Some("host"),
        "p" | "port" => Some("port"),
        "ngl" | "gpu-layers" | "n-gpu-layers" => Some("ngl"),
        "t" | "threads" => Some("threads"),
        "c" | "ctx-size" => Some("ctx-size"),
        "fa" | "flash-attn" => Some("flash-attn"),
        _ => None,
    }
}

fn quote_if_needed(s: &str) -> String {
    if s.is_empty() || s.contains(char::is_whitespace) || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// 构建参数列表（不含可执行文件），并收集校验信息。
pub fn build_args(profile: &Profile) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut overridden: Vec<String> = Vec::new();
    let mut duplicated: Vec<String> = Vec::new();

    let extra = split_args(&profile.extra_params);

    // 统计自定义区参数：覆盖与重复
    let mut seen: HashMap<&'static str, usize> = HashMap::new();
    let mut custom_groups: Vec<&'static str> = Vec::new();
    for tok in &extra {
        if let Some(group) = group_name(tok) {
            if !custom_groups.contains(&group) {
                custom_groups.push(group);
            }
            let entry = seen.entry(group).or_insert(0);
            *entry += 1;
            if *entry == 2 {
                duplicated.push(group.to_string());
            }
        }
    }

    let mut args: Vec<String> = Vec::new();

    let push_form =
        |args: &mut Vec<String>, group: &str, tokens: Vec<String>, overridden: &mut Vec<String>| {
            if custom_groups.contains(&group) {
                if !overridden.iter().any(|o| o == group) {
                    overridden.push(group.to_string());
                }
            } else {
                args.extend(tokens);
            }
        };

    let model = profile.model_path.trim().to_string();
    if model.is_empty() {
        warnings.push("未指定模型路径".into());
    } else {
        push_form(
            &mut args,
            "model",
            vec!["--model".into(), model],
            &mut overridden,
        );
    }

    push_form(
        &mut args,
        "host",
        vec!["--host".into(), profile.host.clone()],
        &mut overridden,
    );
    push_form(
        &mut args,
        "port",
        vec!["--port".into(), profile.port.to_string()],
        &mut overridden,
    );

    if profile.ngl >= 0 {
        push_form(
            &mut args,
            "ngl",
            vec!["-ngl".into(), profile.ngl.to_string()],
            &mut overridden,
        );
    }
    if profile.threads > 0 {
        push_form(
            &mut args,
            "threads",
            vec!["-t".into(), profile.threads.to_string()],
            &mut overridden,
        );
    }
    if profile.ctx_size > 0 {
        push_form(
            &mut args,
            "ctx-size",
            vec!["-c".into(), profile.ctx_size.to_string()],
            &mut overridden,
        );
    }
    if profile.flash_attn {
        push_form(
            &mut args,
            "flash-attn",
            vec!["--flash-attn".into()],
            &mut overridden,
        );
    }

    // 自定义参数原样追加（CLI 惯例：后者覆盖前者）
    args.extend(extra);

    if profile.port < 1024 {
        warnings.push(format!(
            "端口 {} 低于 1024，可能被占用或需要权限",
            profile.port
        ));
    }

    (args, warnings, overridden, duplicated)
}

/// 组装完整预览字符串。
pub fn build_preview(binary_path: &str, args: &[String]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(args.len() + 1);
    parts.push(quote_if_needed(binary_path));
    for a in args {
        parts.push(quote_if_needed(a));
    }
    parts.join(" ")
}

/// 展开 `%VAR%` 与 `${VAR}`；变量未定义时保留原样。
pub fn expand_env_vars(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '%' {
            if let Some(end) = chars[i + 1..].iter().position(|&x| x == '%') {
                let name: String = chars[i + 1..i + 1 + end].iter().collect();
                if !name.is_empty() {
                    if let Ok(val) = std::env::var(&name) {
                        out.push_str(&val);
                        i += end + 2;
                        continue;
                    }
                }
            }
        } else if c == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            if let Some(end) = chars[i + 2..].iter().position(|&x| x == '}') {
                let name: String = chars[i + 2..i + 2 + end].iter().collect();
                if let Ok(val) = std::env::var(&name) {
                    out.push_str(&val);
                    i += end + 3;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// 生成 .bat 文件内容。
fn build_bat(binary: &str, args: &[String], profile: &Profile, absolute: bool) -> String {
    let workdir = if absolute {
        expand_env_vars(&profile.work_dir)
    } else {
        profile.work_dir.clone()
    };
    let mut lines: Vec<String> = vec!["@echo off".into(), "chcp 65001 >nul".into()];
    if !workdir.trim().is_empty() {
        lines.push(format!("cd /d \"{}\"", workdir.trim()));
    }
    for env in &profile.env_vars {
        let env = env.trim();
        if env.is_empty() || env.starts_with('#') || !env.contains('=') {
            continue;
        }
        let env = if absolute {
            expand_env_vars(env)
        } else {
            env.to_string()
        };
        lines.push(format!("set \"{env}\""));
    }
    let final_args: Vec<String> = if absolute {
        args.iter().map(|a| expand_env_vars(a)).collect()
    } else {
        args.to_vec()
    };
    let binary = if absolute {
        expand_env_vars(binary)
    } else {
        binary.to_string()
    };
    let rendered: Vec<String> = final_args.iter().map(|a| quote_if_needed(a)).collect();
    lines.push(format!(
        "{} {}",
        quote_if_needed(&binary),
        rendered.join(" ")
    ));
    lines.join("\r\n")
}

fn current_binary_path() -> Result<String, String> {
    let cfg = load_config()?;
    if cfg.global.binary_path.is_empty() {
        Ok("llama-server.exe".into())
    } else {
        Ok(cfg.global.binary_path)
    }
}

/// 命令构建命令：读取全局 binary_path，输出预览与校验。
#[tauri::command]
pub fn build_command_cmd(profile: Profile) -> Result<CommandPreview, String> {
    let binary_path = current_binary_path()?;
    let (args, warnings, overridden, duplicated) = build_args(&profile);
    let preview = build_preview(&binary_path, &args);
    Ok(CommandPreview {
        args,
        preview,
        warnings,
        overridden,
        duplicated,
    })
}

/// 导出 .bat；absolute=true 时展开 %VAR% / ${VAR} 为绝对路径。
#[tauri::command]
pub fn export_bat_cmd(profile: Profile, path: String, absolute: bool) -> Result<(), String> {
    let binary = current_binary_path()?;
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let (args, _, _, _) = build_args(&profile);
    let content = build_bat(&binary, &args, &profile, absolute);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}
