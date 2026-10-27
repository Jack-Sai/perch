//! 参数系统：从配置方案构建 llama-server 命令行、命令预览与基础校验。
//!
//! M1 范围：常用参数表单（模型/host/端口/-ngl/-t/-c/--flash-attn）
//! + 自定义参数文本区 + 实时命令预览。高级参数折叠区为 M2 内容。

use serde::{Deserialize, Serialize};

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

/// 从 token 提取长参数名（去掉前导 -- 与附加的值）。
fn long_name(token: &str) -> Option<String> {
    if let Some(rest) = token.strip_prefix("--") {
        // 形如 --ctx-size=4096
        let name = rest.split('=').next().unwrap_or(rest);
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

/// 构建参数列表（不含可执行文件），并收集校验信息。
pub fn build_args(profile: &Profile) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut args: Vec<String> = Vec::new();
    let mut warnings = Vec::new();
    let mut overridden = Vec::new();
    let mut duplicated = Vec::new();

    // 常用参数
    if !profile.model_path.is_empty() {
        args.push("--model".into());
        args.push(profile.model_path.clone());
    } else {
        warnings.push("未指定模型路径".into());
    }
    args.push("--host".into());
    args.push(profile.host.clone());
    args.push("--port".into());
    args.push(profile.port.to_string());

    let common_long = ["model", "host", "port", "ngl", "t", "c", "flash-attn"];
    if profile.ngl >= 0 {
        args.push("-ngl".into());
        args.push(profile.ngl.to_string());
    }
    if profile.threads > 0 {
        args.push("-t".into());
        args.push(profile.threads.to_string());
    }
    if profile.ctx_size > 0 {
        args.push("-c".into());
        args.push(profile.ctx_size.to_string());
    }
    if profile.flash_attn {
        args.push("--flash-attn".into());
    }

    // 自定义参数区
    let extra = split_args(&profile.extra_params);
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut i = 0;
    while i < extra.len() {
        let tok = &extra[i];
        if let Some(name) = long_name(tok) {
            if common_long.contains(&name.as_str()) && !overridden.contains(&name) {
                overridden.push(name.clone());
            }
            let entry = seen.entry(name.clone()).or_insert(0);
            *entry += 1;
            if *entry == 2 && !duplicated.contains(&name) {
                duplicated.push(name.clone());
            }
        }
        // 若下一 token 不是以 - 开头，则视为当前参数的值，跳过
        if i + 1 < extra.len() && !extra[i + 1].starts_with('-') {
            args.push(tok.clone());
            args.push(extra[i + 1].clone());
            i += 2;
            continue;
        }
        args.push(tok.clone());
        i += 1;
    }

    if profile.port < 1024 {
        warnings.push(format!("端口 {} 超出推荐范围 1024–65535", profile.port));
    }

    (args, warnings, overridden, duplicated)
}

/// 生成展示用命令行（含可执行文件路径，路径含空格时加引号）。
fn quote_if_needed(s: &str) -> String {
    if s.contains(char::is_whitespace) {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}

/// 组装完整预览字符串。
pub fn build_preview(binary_path: &str, args: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(quote_if_needed(binary_path));
    for a in args {
        parts.push(quote_if_needed(a));
    }
    parts.join(" ")
}

/// 命令构建命令：读取全局 binary_path，输出预览与校验。
#[tauri::command]
pub fn build_command_cmd(profile: Profile) -> Result<CommandPreview, String> {
    let cfg = load_config()?;
    let binary_path = if cfg.global.binary_path.is_empty() {
        "llama-server.exe".to_string()
    } else {
        cfg.global.binary_path.clone()
    };
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
