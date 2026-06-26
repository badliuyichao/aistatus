//! AI 余额监控 · Tauri 后端入口。
//!
//! 对应 legacy `main.py`：建窗口 + 启动后台拉取 + 定时刷新。
//! 但所有 I/O 都在 Rust 侧（密钥/网络不进 webview）。

pub mod commands;
pub mod config;
pub mod data;
mod fetcher;
mod merge;
mod state;

use std::time::Duration;

use config::load_keys;
use state::AppState;

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
            // 启动时触发一次后台拉取（对应 legacy widget.request_refresh）。
            // request_refresh 是关联函数，内部通过 app.state() 自取状态。
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                AppState::request_refresh(handle).await;
            });

            // 60s 定时刷新（对应 legacy main.py 的 QTimer 60_000）
            let handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval =
                    tokio::time::interval(Duration::from_secs(60));
                // 跳过首次立即触发（启动时上面已拉过一次）
                interval.tick().await;
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
