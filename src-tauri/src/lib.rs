mod binary;
mod config;
mod monitor;
mod params;
mod process;
mod single_instance;
mod state;
mod tray;

use std::sync::Arc;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 单实例：重复启动则聚焦已有窗口并退出
    if single_instance::is_secondary_instance() {
        single_instance::focus_primary();
        return;
    }

    let state = Arc::new(state::AppState::new());

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            config::load_config_cmd,
            config::save_config_cmd,
            config::create_profile,
            config::delete_profile,
            config::set_last_profile,
            binary::detect_binary_cmd,
            binary::resolve_binary_cmd,
            params::build_command_cmd,
            process::start_server_cmd,
            process::stop_server_cmd,
            process::get_server_status_cmd,
            process::read_server_log_cmd,
            monitor::get_metrics_cmd,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            tray::create_tray(&handle, &state)?;
            single_instance::start_focus_listener(handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            // 关闭主窗口：依据全局设置隐藏到托盘或直接退出
            if let Ok(cfg) = config::load_config() {
                if cfg.global.close_to_tray && !cfg.global.exit_keep_server {
                    api.prevent_close();
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            }
        }
    });
}
