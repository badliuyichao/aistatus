//! AI 余额监控 · Tauri 后端入口。
//!
//! 对应 legacy `main.py` + widget 的窗口/托盘初始化：
//!   - 建窗口（无边框/置顶/透明，配置在 tauri.conf.json）
//!   - 应用原生毛玻璃（backdrop 模块）
//!   - 锚定屏幕右下角（_anchor_to_bottom_right）
//!   - 系统托盘（显示/退出）
//!   - 启动后台拉取 + 60s 定时

pub mod backdrop;
pub mod commands;
pub mod config;
pub mod data;
mod fetcher;
mod merge;
mod state;

use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition,
};

use config::load_keys;
use state::AppState;

/// 窗口距屏幕右下角的留白（对应 legacy WINDOW_SCREEN_MARGIN）。
const WINDOW_SCREEN_MARGIN: i32 = 20;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_keys = load_keys();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new(initial_keys))
        .invoke_handler(tauri::generate_handler![
            commands::get_services,
            commands::get_keys,
            commands::save_keys,
            commands::refresh_now,
            commands::needs_key_setup,
            commands::open_balance_file,
        ])
        .setup(|app| {
            // ── 窗口初始化：毛玻璃 + 锚定右下角 ──
            if let Some(window) = app.get_webview_window("main") {
                // 原生毛玻璃；失败静默（前端 CSS 降级）
                let _ = backdrop::apply(&window);
                // 锚定主屏右下角（对应 legacy _anchor_to_bottom_right）
                anchor_to_bottom_right(&window);
            }

            // ── 系统托盘：显示 / 退出 ──
            let show_item = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let mut tray = TrayIconBuilder::new()
                .tooltip("AI API 余额监控")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // 双击托盘图标 → 显示窗口（对应 legacy _on_tray_activated）
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        show_main_window(app);
                    }
                });

            // 托盘图标：优先用打包的图标文件
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;

            // ── 启动时触发一次后台拉取（对应 legacy widget.request_refresh）──
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                AppState::request_refresh(handle).await;
            });

            // ── 60s 定时刷新（对应 legacy main.py 的 QTimer 60_000）──
            let handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                interval.tick().await; // 跳过首次立即触发
                loop {
                    interval.tick().await;
                    AppState::request_refresh(handle2.clone()).await;
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 显示主窗口（托盘「显示」/ 双击）。对应 legacy `_show_window`。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 把窗口移到主屏工作区右下角（排除 Dock/任务栏）。
/// 对应 legacy `_anchor_to_bottom_right`。
fn anchor_to_bottom_right(window: &tauri::WebviewWindow) {
    let monitor = match window.current_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let mon_size = monitor.size();
    let mon_pos = monitor.position();
    let win_size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };

    // 工作区右下角 = 屏幕原点 + 屏幕尺寸 - 留白；
    // 屏幕原点可能是负数（多屏），所以要加 mon_pos。
    let x = mon_pos.x as i32 + mon_size.width as i32 - win_size.width as i32 - WINDOW_SCREEN_MARGIN;
    let y = mon_pos.y as i32 + mon_size.height as i32
        - win_size.height as i32
        - WINDOW_SCREEN_MARGIN;
    // 钳制不滑出左上角
    let x = x.max(mon_pos.x as i32);
    let y = y.max(mon_pos.y as i32);
    let _ = window.set_position(PhysicalPosition { x, y });
}
