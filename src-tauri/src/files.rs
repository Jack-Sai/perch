use std::path::Path;

/// 将文本写入用户选择的路径（如监控数据导出）。
#[tauri::command]
pub fn save_text_file_cmd(path: String, contents: String) -> Result<(), String> {
    let target = Path::new(&path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
    }
    std::fs::write(target, contents.as_bytes()).map_err(|e| format!("写入文件失败: {e}"))
}
