//! 数据模型 —— 定义配置文件、运行时快照与前端事件的共享结构。
//!
//! 这些结构同时承担两个方向的序列化：
//!   1. 反序列化 `balance.json`（用户编辑的模板文件，字段最简）
//!   2. 序列化后通过 Tauri event 推给前端（含 `loading` 等运行期字段）
//!
//! serde 用 camelCase，与前端 TypeScript 命名一致；
//! 所有字段 `skip_serializing_if` / 默认值保证缺字段时也能解析（兼容旧 balance.json）。

use serde::{Deserialize, Serialize};

/// 一个配额项（已用/总量），如「5小时限额」。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaItem {
    #[serde(default)]
    pub label: String,

    /// 可选的自定义展示值（如无限额 `∞`）。
    /// 仅影响前端右侧文本，进度条仍使用 used / total。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_value: Option<String>,

    #[serde(default)]
    pub used: f64,

    /// 总量。默认 1.0，避免计算百分比时除零。
    #[serde(default = "default_total")]
    pub total: f64,

    #[serde(default)]
    pub unit: String,

    /// 详情文本（如「3小时20分后重置」）。空字符串表示不显示。
    #[serde(default)]
    pub detail: String,
}

fn default_total() -> f64 {
    1.0
}

impl QuotaItem {
    /// 已用百分比，0-100。total<=0 时返回 0。
    pub fn percentage(&self) -> f64 {
        if self.total <= 0.0 {
            0.0
        } else {
            (self.used / self.total * 100.0).min(100.0)
        }
    }

    /// 剩余量，不小于 0。
    #[allow(dead_code)]
    pub fn remaining(&self) -> f64 {
        (self.total - self.used).max(0.0)
    }
}

/// 一个服务商条目（GLM / DeepSeek / MiniMax）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    #[serde(default)]
    pub name: String,

    /// "quota" | "balance"
    #[serde(default = "default_service_type")]
    pub r#type: String, // `type` 是 Rust 关键字，用 r#type

    #[serde(default)]
    pub icon: String,

    #[serde(default = "default_color")]
    pub color: String,

    /// quota 型的配额项列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<QuotaItem>,

    // ── balance 型字段 ──
    #[serde(default)]
    pub balance: f64,

    #[serde(default = "default_currency")]
    pub currency: String,

    #[serde(default = "default_unit")]
    pub unit: String,

    #[serde(default)]
    pub detail: String,

    /// 无实时数据时（启动中 / 拉取失败）为 true，卡片显示 N/A。
    #[serde(default)]
    pub loading: bool,
}

fn default_service_type() -> String {
    "balance".to_string()
}
fn default_color() -> String {
    "#888888".to_string()
}
fn default_currency() -> String {
    "¥".to_string()
}
fn default_unit() -> String {
    "元".to_string()
}

/// 完整的余额数据（balance.json 顶层结构）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceData {
    #[serde(default = "default_title")]
    pub title: String,

    #[serde(default)]
    pub services: Vec<ServiceInfo>,

    /// 加载时间戳（Unix 秒）。前端据此显示「更新于 …」。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
}

fn default_title() -> String {
    "AI API 余额监控".to_string()
}

impl BalanceData {
    /// 当前 Unix 时间戳（秒）。
    pub fn now_timestamp() -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

// ── fetcher 返回的原始 API 数据（merge 前的中间结构）──
// 三家响应结构不同；各 fetcher 解析后统一转换为这些结构。

/// DeepSeek fetcher 返回的展示数据。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekResult {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub icon: String,
    pub balance: f64,
    pub currency: String,
    pub unit: String,
    pub detail: String,
    pub color: String,
}

/// GLM fetcher 返回的展示数据（带 level 套餐档位）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlmResult {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub icon: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    pub items: Vec<QuotaItem>,
}

/// MiniMax fetcher 返回的展示数据。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinimaxResult {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub icon: String,
    pub color: String,
    pub items: Vec<QuotaItem>,
}

/// 三家拉取结果（任一为 None 表示无 key / 失败 / 无数据）。
#[derive(Clone, Debug, Default)]
pub struct FetchOutcome {
    pub deepseek: Option<DeepSeekResult>,
    pub glm: Option<GlmResult>,
    pub minimax: Option<MinimaxResult>,
}
