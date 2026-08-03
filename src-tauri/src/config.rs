//! config.json 读写 —— 存放三家 API 密钥。
//!
//! 路径：跨平台用户配置目录下的 `aistatus/config.json`
//!   - macOS: ~/Library/Application Support/aistatus/config.json
//!   - Windows: %APPDATA%\aistatus\config.json
//!   - Linux: ~/.config/aistatus/config.json（预留）
//!
//! `dirs::config_dir()` 负责按平台返回符合系统约定的配置目录。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 三家 API 密钥。任一为空字符串表示「未配置该厂商」。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeys {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub deepseek_api_key: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub glm_api_key: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub minimax_api_key: String,
}

impl ApiKeys {
    /// 是否三家都未配置（首次运行判断用）。
    pub fn all_empty(&self) -> bool {
        self.deepseek_api_key.is_empty()
            && self.glm_api_key.is_empty()
            && self.minimax_api_key.is_empty()
    }

    pub fn has_deepseek(&self) -> bool {
        !self.deepseek_api_key.is_empty()
    }
    pub fn has_glm(&self) -> bool {
        !self.glm_api_key.is_empty()
    }
    pub fn has_minimax(&self) -> bool {
        !self.minimax_api_key.is_empty()
    }
}

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

/// 读取 config.json；文件不存在或解析失败时返回空的 ApiKeys。
pub fn load_keys() -> ApiKeys {
    read_json_or_default::<ApiKeys>(&config_path())
}

/// 写入 config.json。自动创建父目录。
pub fn save_keys(keys: &ApiKeys) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(keys)?;
    // 先写临时文件再 rename，避免写入中途崩溃导致 config 损坏。
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, &path)?;
    Ok(())
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
