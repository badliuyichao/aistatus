//! 暴露给前端调用的 Tauri 命令。
//!
//! 包含手动刷新、打开数据文件、主题与悬浮条设置等交互入口。
//! API Key 已改为编译期加密硬编码，不再经由此处配置。

use tauri::{AppHandle, Emitter, Manager, State};

use crate::config;
use crate::data::BalanceData;
use crate::state::AppState;

/// 返回当前已合并的展示数据（含 loading 态）。
/// 前端启动时调一次拿初始数据，之后靠 `services-updated` 事件增量更新。
#[tauri::command]
pub async fn get_services(state: State<'_, AppState>) -> Result<BalanceData, String> {
    Ok(state.snapshot().await)
}

/// 手动触发一次后台刷新（F5 / 右键菜单）。
#[tauri::command]
pub async fn refresh_now(app: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    AppState::request_refresh(app).await;
    Ok(())
}

/// 用系统默认编辑器打开 balance.json。
///
/// 走 tauri-plugin-opener 的系统文件关联（底层 `open` crate）：
/// 用 .json 的默认程序打开（VS Code / 新版记事本等），
/// 而非硬编码 notepad——后者在老版 Windows 上按 GBK 打开无 BOM 的
/// UTF-8，会导致中文乱码并可能污染 balance.json。
#[tauri::command]
pub fn open_balance_file() -> Result<(), String> {
    let path = config::balance_path();
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

/// 读主题偏好（system / dark / light）。前端启动时据此初始化 <html data-theme>。
#[tauri::command]
pub fn get_theme() -> String {
    config::load_theme()
}

/// 保存主题偏好：持久化到 config.json，并广播 theme-changed 让所有窗口
/// （主窗口 + 悬浮条）同步切换。`app.emit` 是全局广播，但发起切换的窗口
/// 自身的 theme store 已即时 apply，收到事件再 apply 一次幂等无副作用。
#[tauri::command]
pub fn save_theme(app: AppHandle, theme: String) -> Result<(), String> {
    if !matches!(theme.as_str(), "system" | "dark" | "light") {
        return Err(format!("invalid theme: {theme}"));
    }
    config::save_theme(&theme).map_err(|e| e.to_string())?;
    let _ = app.emit("theme-changed", theme);
    Ok(())
}

/// 给后台拉取线程用的内部广播：拉取完成后向所有窗口发 `services-updated`。
#[allow(dead_code)]
pub fn emit_services_updated(app: &AppHandle, data: &BalanceData) {
    let _ = app.emit("services-updated", data.clone());
}

/// 保存悬浮球窗口位置（逻辑像素）。前端拖拽结束时调用。
#[tauri::command]
pub fn save_float_position(x: f64, y: f64) -> Result<(), String> {
    config::save_float_position(x, y).map_err(|e| e.to_string())
}

/// 显示 / 隐藏悬浮球窗口（托盘菜单开关用）。
#[tauri::command]
pub fn set_float_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("float") {
        if visible {
            let _ = window.show();
        } else {
            let _ = window.hide();
        }
    }
    Ok(())
}
