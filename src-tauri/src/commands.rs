//! 暴露给前端调用的 Tauri 命令。
//!
//! 对应 legacy `widget.py` 里的交互入口：
//!   - 配置 API Key（`_open_config` / `fetcher.set_key`）
//!   - 手动刷新（`_manual_refresh` / `request_refresh`）
//!   - 打开数据文件编辑（`_open_data_file`）
//!   - 首次无 key 引导（`prompt_for_keys_if_needed`）

use std::process::Command;

use tauri::{AppHandle, Emitter, State};

use crate::config::{self, ApiKeys};
use crate::data::BalanceData;
use crate::state::AppState;

/// 返回当前已合并的展示数据（含 loading 态）。
/// 前端启动时调一次拿初始数据，之后靠 `services-updated` 事件增量更新。
#[tauri::command]
pub async fn get_services(state: State<'_, AppState>) -> Result<BalanceData, String> {
    Ok(state.snapshot().await)
}

/// 读已保存的 key（脱敏：只返回是否存在，不回传明文给前端配置框回填时用明文）。
/// 注意：配置框需要回填明文让用户编辑，所以这里返回明文（仅本机 webview 内存）。
#[tauri::command]
pub fn get_keys() -> ApiKeys {
    config::load_keys()
}

/// 保存三家 key 并立即触发一次后台刷新。
/// 对应 legacy 配置框「保存并刷新」。
#[tauri::command]
pub async fn save_keys(
    app: AppHandle,
    state: State<'_, AppState>,
    deepseek: String,
    glm: String,
    minimax: String,
) -> Result<(), String> {
    let keys = ApiKeys {
        deepseek_api_key: deepseek.trim().to_string(),
        glm_api_key: glm.trim().to_string(),
        minimax_api_key: minimax.trim().to_string(),
    };
    config::save_keys(&keys).map_err(|e| e.to_string())?;
    // 更新运行期 key 副本 + 立即刷新
    state.update_keys(keys).await;
    AppState::request_refresh(app).await;
    Ok(())
}

/// 手动触发一次后台刷新（F5 / 右键菜单）。
/// 对应 legacy `request_refresh`。
#[tauri::command]
pub async fn refresh_now(app: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    AppState::request_refresh(app).await;
    Ok(())
}

/// 是否还有任何一家 key 未配置（首次运行引导用）。
/// 对应 legacy `prompt_for_keys_if_needed`。
#[tauri::command]
pub fn needs_key_setup() -> bool {
    config::load_keys().all_empty()
}

/// 用系统默认编辑器打开 balance.json。
/// 对应 legacy `_open_data_file`（Win: notepad；mac: `open -t`）。
#[tauri::command]
pub fn open_balance_file() -> Result<(), String> {
    let path = config::balance_path();
    let r = if cfg!(target_os = "windows") {
        Command::new("notepad").arg(&path).spawn()
    } else if cfg!(target_os = "macos") {
        // `open -t` 用默认纯文本编辑器打开
        Command::new("open").args(["-t", path.to_str().unwrap_or("")]).spawn()
    } else {
        // Linux 预留：xdg-open
        Command::new("xdg-open").arg(&path).spawn()
    };
    r.map(|_| ()).map_err(|e| e.to_string())
}

/// 给后台拉取线程用的内部广播：拉取完成后向所有窗口发 `services-updated`。
#[allow(dead_code)]
pub fn emit_services_updated(app: &AppHandle, data: &BalanceData) {
    let _ = app.emit("services-updated", data.clone());
}
