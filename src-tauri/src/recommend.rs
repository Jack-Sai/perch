//! 推荐参数：读取 GGUF 元数据并结合 GPU 显存给出 -ngl / -c / -t / --flash-attn 建议。

use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Serialize;

use crate::config::Profile;
use crate::monitor::gpu_info;

const PREFIX_LIMIT: u64 = 32 * 1024 * 1024;
const MAX_KV: u64 = 200_000;
const MAX_ARRAY: u64 = 50_000_000;

#[derive(Debug, Clone, Default)]
struct GgufMeta {
    arch: Option<String>,
    file_type: Option<u32>,
    context_length: Option<u64>,
    block_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendationItem {
    pub key: String,
    pub value: String,
    pub reason_key: String,
    pub reason_args: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub items: Vec<RecommendationItem>,
    pub facts: Vec<String>,
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        if end > self.data.len() {
            return None;
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Some(slice)
    }

    fn u32(&mut self) -> Option<u32> {
        let b = self.take(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Option<u64> {
        let b = self.take(8)?;
        Some(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn string(&mut self) -> Option<String> {
        let len = self.u64()?;
        if len > 16 * 1024 * 1024 {
            return None;
        }
        let b = self.take(len as usize)?;
        Some(String::from_utf8_lossy(b).into_owned())
    }

    fn skip_value(&mut self, ty: u32, depth: u32) -> Option<()> {
        if depth > 4 {
            return None;
        }
        let size = match ty {
            0 | 1 | 7 => 1,
            2 | 3 => 2,
            4 | 5 | 6 => 4,
            10 | 11 | 12 => 8,
            8 => {
                self.string()?;
                return Some(());
            }
            9 => {
                let elem = self.u32()?;
                let count = self.u64()?;
                if count > MAX_ARRAY {
                    return None;
                }
                for _ in 0..count {
                    self.skip_value(elem, depth + 1)?;
                }
                return Some(());
            }
            _ => return None,
        };
        self.take(size)?;
        Some(())
    }
}

fn parse_meta(data: &[u8]) -> Option<GgufMeta> {
    let mut c = Cursor::new(data);
    if c.take(4)? != b"GGUF" {
        return None;
    }
    let version = c.u32()?;
    if !(2..=3).contains(&version) {
        return None;
    }
    let _tensor_count = c.u64()?;
    let kv_count = c.u64()?;
    if kv_count > MAX_KV {
        return None;
    }

    let mut meta = GgufMeta::default();
    for _ in 0..kv_count {
        let key = c.string()?;
        let ty = c.u32()?;
        match key.as_str() {
            "general.architecture" if ty == 8 => meta.arch = Some(c.string()?),
            "general.file_type" if ty == 4 => meta.file_type = Some(c.u32()?),
            "general.parameter_count" if ty == 10 => {
                let _ = c.u64()?;
            }
            k if ty == 4 && k.ends_with(".context_length") => {
                meta.context_length = Some(c.u32()? as u64)
            }
            k if ty == 10 && k.ends_with(".context_length") => meta.context_length = Some(c.u64()?),
            k if ty == 4 && k.ends_with(".block_count") => meta.block_count = Some(c.u32()? as u64),
            k if ty == 10 && k.ends_with(".block_count") => meta.block_count = Some(c.u64()?),
            _ => c.skip_value(ty, 0)?,
        }
    }
    Some(meta)
}

fn read_meta(path: &Path) -> Result<(GgufMeta, u64), String> {
    let mut file = File::open(path).map_err(|e| format!("无法打开模型文件: {e}"))?;
    let file_size = file
        .metadata()
        .map_err(|e| format!("读取模型信息失败: {e}"))?
        .len();
    let mut buf = Vec::new();
    file.by_ref()
        .take(PREFIX_LIMIT)
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取模型头部失败: {e}"))?;
    let meta = parse_meta(&buf).ok_or_else(|| "不是有效的 GGUF 文件或元数据已损坏".to_string())?;
    Ok((meta, file_size))
}

fn quant_name(file_type: Option<u32>) -> Option<String> {
    let name = match file_type? {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        4 => "Q5_0",
        5 => "Q5_1",
        6 => "Q8_0",
        7 => "Q5_K_S",
        8 => "Q5_K_M",
        9 => "Q6_K",
        10 => "IQ2_XXS",
        11 => "IQ2_XS",
        12 => "Q2_K_S",
        13 => "IQ3_XS",
        14 => "IQ3_XXS",
        15 => "IQ1_S",
        16 => "IQ4_NL",
        17 => "IQ3_S",
        18 => "IQ3_M",
        19 => "IQ2_S",
        20 => "IQ2_M",
        21 => "Q4_K_S",
        22 => "Q4_K_M",
        23 => "IQ4_XS",
        24 => "IQ1_M",
        25 => "BF16",
        other => return Some(format!("FT{other}")),
    };
    Some(name.into())
}

fn item(key: &str, value: impl ToString, reason_key: &str) -> RecommendationItem {
    RecommendationItem {
        key: key.into(),
        value: value.to_string(),
        reason_key: reason_key.into(),
        reason_args: BTreeMap::new(),
    }
}

fn with_args(mut item: RecommendationItem, args: &[(&str, String)]) -> RecommendationItem {
    for (k, v) in args {
        item.reason_args.insert((*k).into(), v.clone());
    }
    item
}

fn gib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
}

#[tauri::command]
pub fn recommend_cmd(profile: Profile) -> Result<Recommendation, String> {
    let path = Path::new(&profile.model_path);
    if profile.model_path.trim().is_empty() || !path.is_file() {
        return Err("请先选择有效的 GGUF 模型文件".into());
    }
    let (meta, file_size) = read_meta(path)?;
    let gpu_index = crate::config::load_config()
        .map(|c| c.global.gpu_index)
        .unwrap_or(0);
    let gpu = gpu_info(gpu_index);
    let vram = gpu.mem_total.unwrap_or(0);

    let mut facts = Vec::new();
    let quant = quant_name(meta.file_type);
    let mut file_fact = format!("{} GiB", gib(file_size));
    if let Some(q) = &quant {
        file_fact = format!("{q} · {file_fact}");
    }
    if let Some(ctx) = meta.context_length {
        file_fact.push_str(&format!(" · ctx {ctx}"));
    }
    if let Some(blocks) = meta.block_count {
        file_fact.push_str(&format!(" · {blocks} layers"));
    }
    if let Some(arch) = &meta.arch {
        file_fact.push_str(&format!(" · {arch}"));
    }
    facts.push(file_fact);
    facts.push(match (&gpu.name, vram) {
        (Some(name), v) if v > 0 => format!("{name} · {} GiB VRAM", gib(v)),
        _ => "未检测到 NVIDIA GPU（CPU 模式）".into(),
    });

    let mut items = Vec::new();

    let train_ctx = meta.context_length.unwrap_or(0);
    let vram_cap = if vram >= 15 * 1024 * 1024 * 1024 {
        32768
    } else if vram >= 7 * 1024 * 1024 * 1024 {
        16384
    } else if vram >= 5 * 1024 * 1024 * 1024 {
        8192
    } else if vram > 0 {
        4096
    } else {
        4096
    };
    let ctx = if train_ctx > 0 {
        train_ctx.min(vram_cap).max(2048)
    } else {
        vram_cap
    };
    items.push(with_args(
        item("ctx_size", ctx, "rec.ctx"),
        &[
            ("train", train_ctx.to_string()),
            ("chosen", ctx.to_string()),
        ],
    ));

    if let Some(blocks) = meta.block_count.filter(|b| *b > 0) {
        if vram > 0 {
            let usable = vram as f64 * 0.85;
            let per_layer = file_size as f64 / blocks as f64 * 1.15;
            let fit = (usable / per_layer.max(1.0)).floor() as u64;
            if fit >= blocks {
                items.push(with_args(
                    item("ngl", 999, "rec.nglAll"),
                    &[("layers", blocks.to_string())],
                ));
            } else {
                items.push(with_args(
                    item("ngl", fit.max(0), "rec.nglPartial"),
                    &[("fit", fit.to_string()), ("layers", blocks.to_string())],
                ));
            }
        } else {
            items.push(item("ngl", 0, "rec.nglCpu"));
        }
    } else if vram > 0 {
        items.push(item("ngl", 999, "rec.nglAllUnknown"));
    }

    let logical = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    let cores = (logical / 2).clamp(2, 16);
    items.push(with_args(
        item("threads", cores, "rec.threads"),
        &[("cores", cores.to_string())],
    ));

    if vram > 0 {
        items.push(item("flash_attn", "true", "rec.flash"));
    }

    Ok(Recommendation { items, facts })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(v: &mut Vec<u8>, n: u32) {
        v.extend_from_slice(&n.to_le_bytes());
    }
    fn put_u64(v: &mut Vec<u8>, n: u64) {
        v.extend_from_slice(&n.to_le_bytes());
    }
    fn put_str(v: &mut Vec<u8>, s: &str) {
        put_u64(v, s.len() as u64);
        v.extend_from_slice(s.as_bytes());
    }

    fn synthetic() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"GGUF");
        put_u32(&mut b, 3);
        put_u64(&mut b, 0);
        put_u64(&mut b, 5);
        put_str(&mut b, "general.architecture");
        put_u32(&mut b, 8);
        put_str(&mut b, "llama");
        put_str(&mut b, "general.file_type");
        put_u32(&mut b, 4);
        put_u32(&mut b, 22);
        put_str(&mut b, "llama.context_length");
        put_u32(&mut b, 4);
        put_u32(&mut b, 8192);
        put_str(&mut b, "llama.block_count");
        put_u32(&mut b, 4);
        put_u32(&mut b, 32);
        put_str(&mut b, "llama.tensor_types");
        put_u32(&mut b, 9);
        put_u32(&mut b, 4);
        put_u64(&mut b, 2);
        put_u32(&mut b, 1);
        put_u32(&mut b, 6);
        b
    }

    #[test]
    fn parses_metadata() {
        let meta = parse_meta(&synthetic()).expect("metadata");
        assert_eq!(meta.arch.as_deref(), Some("llama"));
        assert_eq!(meta.file_type, Some(22));
        assert_eq!(meta.context_length, Some(8192));
        assert_eq!(meta.block_count, Some(32));
    }

    #[test]
    fn rejects_non_gguf() {
        assert!(parse_meta(b"NOTGGUF").is_none());
    }

    #[test]
    fn maps_quant_names() {
        assert_eq!(quant_name(Some(22)).as_deref(), Some("Q4_K_M"));
        assert_eq!(quant_name(Some(999)).as_deref(), Some("FT999"));
        assert_eq!(quant_name(None), None);
    }
}
