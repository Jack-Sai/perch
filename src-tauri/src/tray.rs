//! 系统托盘：状态展示、启动/停止、打开窗口与 Web UI、切换配置、退出。
//!
//! 图标颜色随状态变更事件切换：运行=绿、错误=红、其余=灰；菜单随状态与配置刷新。

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Listener, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::config::{find_profile, load_config, save_config};
use crate::process::{get_status, start_server, stop_server};
use crate::state::{AppState, ServerStatus};

fn status_label(status: &ServerStatus, zh: bool) -> String {
    match (status, zh) {
        (ServerStatus::Idle, true) => "已停止".into(),
        (ServerStatus::Idle, false) => "Stopped".into(),
        (ServerStatus::Launching, true) => "启动中".into(),
        (ServerStatus::Launching, false) => "Starting".into(),
        (ServerStatus::Running, true) => "运行中".into(),
        (ServerStatus::Running, false) => "Running".into(),
        (ServerStatus::Stopping, true) => "停止中".into(),
        (ServerStatus::Stopping, false) => "Stopping".into(),
        (ServerStatus::Error(_), true) => "错误".into(),
        (ServerStatus::Error(_), false) => "Error".into(),
    }
}

/// 依据最新配置与运行状态重建托盘菜单。
pub fn refresh(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let cfg = load_config().unwrap_or_else(|_| crate::config::default_config());
    let zh = cfg.global.language != "en-US";
    let status = get_status(state);
    let can_start = matches!(status, ServerStatus::Idle | ServerStatus::Error(_));
    let can_stop = matches!(status, ServerStatus::Running | ServerStatus::Launching);

    let result = (|| -> tauri::Result<()> {
        let status_item = MenuItem::with_id(
            app,
            "status",
            format!(
                "{}: {}",
                if zh { "状态" } else { "Status" },
                status_label(&status, zh)
            ),
            false,
            None::<&str>,
        )?;
        let start = MenuItem::with_id(
            app,
            "start",
            if zh { "启动" } else { "Start" },
            can_start,
            None::<&str>,
        )?;
        let stop = MenuItem::with_id(
            app,
            "stop",
            if zh { "停止" } else { "Stop" },
            can_stop,
            None::<&str>,
        )?;
        let show = MenuItem::with_id(
            app,
            "show",
            if zh { "打开主窗口" } else { "Open window" },
            true,
            None::<&str>,
        )?;
        let webui = MenuItem::with_id(
            app,
            "webui",
            if zh { "打开 Web UI" } else { "Open Web UI" },
            matches!(status, ServerStatus::Running),
            None::<&str>,
        )?;

        let profiles_menu =
            Submenu::new(app, if zh { "切换配置" } else { "Switch profile" }, true)?;
        if cfg.profiles.is_empty() {
            let empty = MenuItem::with_id(
                app,
                "no-profiles",
                if zh { "暂无配置" } else { "No profiles" },
                false,
                None::<&str>,
            )?;
            profiles_menu.append(&empty)?;
        } else {
            for p in &cfg.profiles {
                let item = CheckMenuItem::with_id(
                    app,
                    format!("profile:{}", p.name),
                    &p.name,
                    true,
                    p.name == cfg.global.last_profile,
                    None::<&str>,
                )?;
                profiles_menu.append(&item)?;
            }
        }

        let exit = MenuItem::with_id(
            app,
            "exit",
            if zh { "退出" } else { "Quit" },
            true,
            None::<&str>,
        )?;

        let menu = Menu::with_items(
            app,
            &[
                &status_item,
                &PredefinedMenuItem::separator(app)?,
                &start,
                &stop,
                &profiles_menu,
                &PredefinedMenuItem::separator(app)?,
                &show,
                &webui,
                &PredefinedMenuItem::separator(app)?,
                &exit,
            ],
        )?;

        if let Some(tray) = state.tray.lock().unwrap().as_ref() {
            let _ = tray.set_menu(Some(menu));
            let _ = tray.set_tooltip(Some(format!("Perch · {}", status_label(&status, zh))));
        }
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("刷新托盘菜单失败: {e}");
    }
}

/// 创建托盘并注册状态订阅。
pub fn create_tray(app: &tauri::AppHandle, state: &Arc<AppState>) -> tauri::Result<()> {
    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(Image::from_bytes(include_bytes!("../icons/tray_gray.png"))?)
        .tooltip("Perch")
        .on_menu_event(|app, event| handle_menu(app, event))
        .build(app)?;

    *state.tray.lock().unwrap() = Some(tray);
    refresh(app, state);

    // 订阅状态变更：切换图标颜色并刷新菜单可用状态
    let state2 = Arc::clone(state);
    let app_for_listen = app.clone();
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
        let state_for_menu = Arc::clone(&state2);
        let app_for_menu = app_for_listen.clone();
        let _ = app_for_listen.run_on_main_thread(move || {
            refresh(&app_for_menu, &state_for_menu);
        });
        let _ = event;
    });

    Ok(())
}

fn handle_menu(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    let state = app.state::<Arc<AppState>>();
    let id = event.id().as_ref();
    match id {
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
            let cfg = load_config().unwrap_or_else(|_| crate::config::default_config());
            if cfg.global.exit_keep_server {
                app.exit(0);
            } else {
                let handle = app.clone();
                let arc = state.inner().clone();
                std::thread::spawn(move || {
                    crate::process::shutdown_blocking(&arc, std::time::Duration::from_secs(30));
                    let _ = handle.exit(0);
                });
            }
        }
        other if other.starts_with("profile:") => {
            let name = &other["profile:".len()..];
            if let Ok(mut cfg) = load_config() {
                if cfg.profiles.iter().any(|p| p.name == name) {
                    cfg.global.last_profile = name.to_string();
                    let _ = save_config(&cfg);
                    let _ = app.emit("perch-config-changed", ());
                    refresh(app, state.inner());
                }
            }
        }
        _ => {}
    }
}
