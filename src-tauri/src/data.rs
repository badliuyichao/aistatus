//! 数据模型 —— 定义配置结构与前端事件的共享结构。
//!
//! 这些结构序列化后通过 Tauri event 推给前端 / `get_services` 返回，
//! serde 用 camelCase，与前端 TypeScript 命名一致；
//! 所有字段带默认值，缺字段也能解析。

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

/// 一个服务商实例卡片（同品牌可有多张，靠 id 区分）。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    /// 实例稳定 ID（config.json plans 的 id），前端 each key / 显隐匹配用。
    #[serde(default)]
    pub id: String,

    /// 服务商稳定 ID：glm / minimax / mimo / deepseek。
    /// 前端图标 / 配色 / 悬浮条分组按它匹配，不依赖显示名。
    #[serde(default)]
    pub vendor: String,

    /// 展示名（品牌名；GLM 带套餐后缀「GLM 智谱AI · {LEVEL}」）。
    #[serde(default)]
    pub name: String,

    /// 用户命名的实例标签（可空），区分同品牌实例；前端拼在名称后展示。
    #[serde(default)]
    pub label: String,

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

    /// 错误占位（如 API Key 无效 / 服务繁忙）：金额给 0 仅作占位，
    /// 前端据此隐藏「安全/预警/危险」余额状态，避免误判。
    #[serde(default)]
    pub placeholder: bool,
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

/// 完整的展示数据（快照 / services-updated 事件负载）。
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

/// 单个 plan 实例的拉取结果。各 fetcher 直接产出（凭据检查 + HTTP + 解析），
/// snapshot 模块把 Ok 的 ServiceInfo 补上 id / vendor / label 后装入快照：
///   - Skipped：未配置凭据 → 不出卡片（对齐旧 MiMo/DeepSeek 的 NotConfigured）
///   - Failed：网络 / HTTP / 解析失败 → 卡片标 loading（显示 N/A）
///   - Ok：成功数据，或明确错误占位（如 401「API Key 无效」、Cookie 失效提示）
#[derive(Clone, Debug, Default)]
pub enum PlanFetch {
    #[default]
    Skipped,
    Failed,
    Ok(ServiceInfo),
}
