//! 暴露给前端调用的 Tauri 命令。
//!
//! 包含手动刷新、打开数据文件、主题与悬浮条设置等交互入口。
//! API Key 已改为编译期加密硬编码，不再经由此处配置；
//! 例外：MiMo 的登录 Cookie 接口不接受 API Key，由此处运行期配置。

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

// ── MiMo Cookie（运行期配置）──

/// 读 MiMo Cookie（设置对话框回显用）。空串表示未配置。
#[tauri::command]
pub fn get_mimo_cookie() -> String {
    config::load_mimo_cookie()
}

/// 保存 MiMo Cookie：宽容清洗 → 持久化到 config.json → 立即触发刷新验证。
/// 清洗后为空串表示清除配置（未配置时主界面不显示 MiMo 卡片）。
#[tauri::command]
pub async fn save_mimo_cookie(app: AppHandle, cookie: String) -> Result<(), String> {
    let normalized = normalize_cookie(&cookie);
    config::save_mimo_cookie(&normalized).map_err(|e| e.to_string())?;
    AppState::request_refresh(app).await;
    Ok(())
}

/// 宽容清洗用户粘贴的 Cookie 输入。
///
/// 接受三种从浏览器 DevTools 复制的形态：
///   1. 纯 Cookie 值：`a=1; b=2`
///   2. 带请求头前缀：`Cookie: a=1; b=2`
///   3. cURL 命令整段：`curl 'https://…' -H 'Cookie: a=1; b=2' …`
/// 换行折叠、分号间距统一（DevTools「复制值」有时带换行）。
/// cookie 值内不会出现分号（RFC 6265），按分号切分安全。
pub(crate) fn normalize_cookie(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let extracted: String = if let Some(rest) = strip_leading_cookie_header(trimmed) {
        rest.to_string()
    } else if let Some(v) = extract_cookie_from_curl(trimmed) {
        v
    } else {
        trimmed.to_string()
    };

    extracted
        .split([';', '\r', '\n'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("; ")
}

/// 行首 `Cookie:` 前缀（大小写不敏感）→ 去掉前缀返回余下值。
fn strip_leading_cookie_header(s: &str) -> Option<&str> {
    if s.len() >= 7 && s[..7].eq_ignore_ascii_case("cookie:") {
        Some(s[7..].trim_start())
    } else {
        None
    }
}

/// cURL 整段中提取 Cookie 头的值：定位最后一个 `cookie:`（不区分大小写），
/// 取到闭合引号或串尾。rfind 避开 URL 中恰好含 "cookie:" 的极端情况
/// （header 一定在 URL 之后）。
fn extract_cookie_from_curl(s: &str) -> Option<String> {
    let lower = s.to_lowercase();
    let idx = lower.rfind("cookie:")?;
    let rest = &s[idx + "cookie:".len()..];
    let end = rest.find(['\'', '"']).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_cookie;

    #[test]
    fn normalize_plain_value() {
        assert_eq!(normalize_cookie("a=1; b=2"), "a=1; b=2");
    }

    #[test]
    fn normalize_strips_header_prefix() {
        assert_eq!(normalize_cookie("Cookie: a=1; b=2"), "a=1; b=2");
        assert_eq!(normalize_cookie("cookie: a=1"), "a=1");
        assert_eq!(normalize_cookie("COOKIE: a=1"), "a=1");
    }

    #[test]
    fn normalize_extracts_from_curl() {
        let curl = "curl 'https://platform.xiaomimimo.com/api/v1/tokenPlan/usage' \
                    -H 'Accept: application/json' \
                    -H 'Cookie: sid=abc; token=xyz'";
        assert_eq!(normalize_cookie(curl), "sid=abc; token=xyz");
    }

    #[test]
    fn normalize_folds_newlines_and_spacing() {
        assert_eq!(normalize_cookie("a=1;\r\nb=2"), "a=1; b=2");
        assert_eq!(normalize_cookie("a=1 ;  b=2 ;"), "a=1; b=2");
        assert_eq!(normalize_cookie("a=1 ;\n\n b=2"), "a=1; b=2");
    }

    #[test]
    fn normalize_empty_clears() {
        assert_eq!(normalize_cookie(""), "");
        assert_eq!(normalize_cookie("   \n  "), "");
        assert_eq!(normalize_cookie("Cookie:"), "");
    }
}
