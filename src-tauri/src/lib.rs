mod api_server;
mod binary;
mod config;
mod files;
mod monitor;
mod params;
mod port;
mod process;
mod recommend;
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
    let setup_state = state.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            config::load_config_cmd,
            config::save_config_cmd,
            config::create_profile,
            config::delete_profile,
            config::set_last_profile,
            config::export_config_cmd,
            config::import_config_cmd,
            binary::detect_binary_cmd,
            binary::resolve_binary_cmd,
            binary::inspect_binary_cmd,
            binary::rescan_help_cmd,
            port::check_port_cmd,
            port::find_free_port_cmd,
            params::build_command_cmd,
            params::export_bat_cmd,
            process::start_server_cmd,
            process::stop_server_cmd,
            process::get_server_status_cmd,
            process::read_server_log_cmd,
            monitor::get_metrics_cmd,
            files::save_text_file_cmd,
            recommend::recommend_cmd,
            api_server::api_status_cmd,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            tray::create_tray(&handle, &setup_state)?;
            single_instance::start_focus_listener(handle);
            api_server::sync_server(Arc::clone(&setup_state));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(move |app_handle, event| {
        if let tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            // 关闭主窗口：隐藏到托盘，或按设置停止 server 后退出 / 保留 server 退出
            let cfg = config::load_config().unwrap_or_else(|_| config::default_config());
            if cfg.global.close_to_tray {
                api.prevent_close();
                if let Some(w) = app_handle.get_webview_window("main") {
                    let _ = w.hide();
                }
            } else if !cfg.global.exit_keep_server {
                api.prevent_close();
                let handle = app_handle.clone();
                let st = Arc::clone(&state);
                std::thread::spawn(move || {
                    process::shutdown_blocking(&st, std::time::Duration::from_secs(30));
                    let _ = handle.exit(0);
                });
            }
        }
    });
}
