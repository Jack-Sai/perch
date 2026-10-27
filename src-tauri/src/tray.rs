//! 系统托盘：菜单（启动/停止/打开窗口/打开 Web UI/退出）与运行状态图标切换。
//!
//! 图标颜色随状态变更事件切换：运行=绿、错误=红、其余=灰。

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Listener, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{find_profile, load_config};
use crate::process::{start_server, stop_server};
use crate::state::AppState;

/// 创建托盘并注册状态订阅。
pub fn create_tray(app: &tauri::AppHandle, state: &Arc<AppState>) -> tauri::Result<()> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "start", "启动", true, None::<&str>)?,
            &MenuItem::with_id(app, "stop", "停止", true, None::<&str>)?,
            &MenuItem::with_id(app, "show", "打开主窗口", true, None::<&str>)?,
            &MenuItem::with_id(app, "webui", "打开 Web UI", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "exit", "退出", true, None::<&str>)?,
        ],
    )?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(Image::from_bytes(include_bytes!("../icons/tray_gray.png"))?)
        .tooltip("Perch")
        .menu(&menu)
        .on_menu_event(|app, event| handle_menu(app, event))
        .build(app)?;

    *state.tray.lock().unwrap() = Some(tray);

    // 订阅状态变更，切换图标颜色
    let state2 = state.clone();
    app.listen("perch-status", move |event| {
        #[derive(serde::Deserialize)]
        struct Payload {
            visual: String,
        }
        if let Ok(p) = serde_json::from_str::<Payload>(event.payload()) {
            let bytes: &[u8] = match p.visual.as_str() {
                "green" => include_bytes!("../icons/tray_green.png"),
                "red" => include_bytes!("../icons/tray_red.png"),
                _ => include_bytes!("../icons/tray_gray.png"),
            };
            if let Ok(img) = Image::from_bytes(bytes) {
                if let Some(t) = state2.tray.lock().unwrap().as_ref() {
                    let _ = t.set_icon(Some(img));
                }
            }
        }
    });

    Ok(())
}

fn handle_menu(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    let state = app.state::<Arc<AppState>>();
    match event.id().as_ref() {
        "start" => {
            if let Ok(cfg) = load_config() {
                if let Some(p) = find_profile(&cfg, &cfg.global.last_profile).cloned() {
                    let _ = start_server(app, state.inner(), p);
                }
            }
        }
        "stop" => {
            let _ = stop_server(app, state.inner());
        }
        "show" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "webui" => {
            if let Ok(cfg) = load_config() {
                let port = find_profile(&cfg, &cfg.global.last_profile)
                    .map(|p| p.port)
                    .unwrap_or(8080);
                let _ = app
                    .opener()
                    .open_url(format!("http://127.0.0.1:{port}"), None::<&str>);
            }
        }
        "exit" => {
            app.exit(0);
        }
        _ => {}
    }
}
