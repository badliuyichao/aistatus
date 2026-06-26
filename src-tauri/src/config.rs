//! config.json 读写 —— 存放三家 API 密钥。
//!
//! 对应 legacy Python 各 fetcher 的 `load_config()` / `save_config()`，
//! 以及 `paths.py` 的 `data_dir()`（frozen 分支）。
//!
//! 路径：跨平台用户配置目录下的 `aistatus/config.json`
//!   - macOS: ~/Library/Application Support/aistatus/config.json
//!   - Windows: %APPDATA%\aistatus\config.json
//!   - Linux: ~/.config/aistatus/config.json（预留）
//!
//! 这与 legacy `%APPDATA%\aistatus` 的 Windows 路径语义一致；
//! `dirs::config_dir()` 在各平台返回的就是这套约定目录。

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 三家 API 密钥。任一为空字符串表示「未配置该厂商」。
/// 对应 Python config.json 的三个字段。
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
    /// 是否三家都未配置（首次运行判断用，对应 Python `prompt_for_keys_if_needed`）。
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
/// 对应 Python `data_dir()`（frozen 分支）。
pub fn data_dir() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("aistatus")
}

/// config.json 完整路径。
pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

/// balance.json 完整路径（首次运行从 resource 模板拷出）。
pub fn balance_path() -> PathBuf {
    data_dir().join("balance.json")
}

/// 读取 config.json；文件不存在或解析失败返回空的 ApiKeys（不报错，
/// 对应 Python `load_config` 的 try/except 兜底）。
pub fn load_keys() -> ApiKeys {
    read_json_or_default::<ApiKeys>(&config_path())
}

/// 写入 config.json。自动创建父目录。
/// 对应 Python `save_config`。
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
