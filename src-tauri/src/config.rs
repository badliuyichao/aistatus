//! config.json 读写 —— 存放主题偏好、悬浮条位置等用户设置。
//!
//! API Key 不在此文件管理（已改为编译期加密硬编码，见 secrets 模块）。
//!
//! 路径：跨平台用户配置目录下的 `aistatus/config.json`
//!   - macOS: ~/Library/Application Support/aistatus/config.json
//!   - Windows: %APPDATA%\aistatus\config.json
//!   - Linux: ~/.config/aistatus/config.json（预留）
//!
//! `dirs::config_dir()` 负责按平台返回符合系统约定的配置目录。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// 用户数据根目录：`<config_dir>/aistatus/`。
pub fn data_dir() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("aistatus")
}

/// config.json 完整路径。
pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

/// balance.json 完整路径（首次运行从内嵌模板创建）。
pub fn balance_path() -> PathBuf {
    data_dir().join("balance.json")
}

/// 通用：读 JSON 文件，失败返回类型的 default。
pub(crate) fn read_json_or_default<T: for<'de> Deserialize<'de> + Default>(
    path: &Path,
) -> T {
    match fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(_) => T::default(),
    }
}

#[allow(dead_code)]
pub fn ensure_data_dir() -> std::io::Result<()> {
    fs::create_dir_all(data_dir())
}

// ── 主题偏好（config.json 的 theme 字段，与 API key 平级）──

/// 读主题偏好；不存在 / 损坏 / 非法值返回 "system"。
/// 对应前端 Theme = "system" | "dark" | "light"。
pub fn load_theme() -> String {
    let value = read_config_value();
    let t = value
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("system");
    if matches!(t, "system" | "dark" | "light") {
        t.to_string()
    } else {
        "system".to_string()
    }
}

/// 写主题偏好：读-改-写（保留 API key 等其他字段）+ 原子 tmp+rename。
pub fn save_theme(theme: &str) -> std::io::Result<()> {
    let path = config_path();
    let mut value = read_config_value();
    if let Some(obj) = value.as_object_mut() {
        obj.insert("theme".into(), serde_json::json!(theme));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&value)?)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// 读 config.json 为 serde_json::Value；不存在 / 损坏 / 非 object 返回空 object。
/// 供 theme 的读-改-写用，与 load_keys 的强类型解析互不干扰。
fn read_config_value() -> serde_json::Value {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .filter(|v: &serde_json::Value| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

// ── 悬浮球窗口位置（config.json 的 floatWindow 字段，逻辑像素）──

/// 读悬浮球上次位置（逻辑像素 x, y）；不存在 / 损坏返回 None。
pub fn load_float_position() -> Option<(f64, f64)> {
    let v = read_config_value();
    let obj = v.get("floatWindow")?.as_object()?;
    let x = obj.get("x")?.as_f64()?;
    let y = obj.get("y")?.as_f64()?;
    Some((x, y))
}

/// 写悬浮球位置：读-改-写（保留其他字段）+ 原子 tmp+rename。
pub fn save_float_position(x: f64, y: f64) -> std::io::Result<()> {
    let path = config_path();
    let mut value = read_config_value();
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "floatWindow".into(),
            serde_json::json!({ "x": x, "y": y }),
        );
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&value)?)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}
