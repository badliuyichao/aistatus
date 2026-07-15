//! GLM 智谱AI Coding Plan 用量查询。
//!
//! URL:    https://open.bigmodel.cn/api/monitor/usage/quota/limit
//! Auth:   Bearer <api_key>
//! 返回配额型（type: quota），带 level 套餐档位。
//!
//! 关键解析逻辑：
//!   - body.code != 200 或无 data → None
//!   - data.limits 为空 → None
//!   - limit.type == "TIME_LIMIT" 且 usage>0 → 「工具调用」次条（绝对值，追加末尾）
//!   - limit.type == "TOKENS_LIMIT"：
//!       unit==3,num==5 → 「5小时限额」
//!       unit==6,num==1 → 「周限额」
//!       否则          → 「Token限额」
//!     有 usage 绝对值 → tokens 条；否则 → 百分比条
//!   - level 非空 → 卡片标题显示「GLM 智谱AI · {LEVEL 大写}」

use reqwest::header::{ACCEPT, AUTHORIZATION};

use crate::data::{GlmResult, QuotaItem};

const GLM_QUOTA_URL: &str = "https://open.bigmodel.cn/api/monitor/usage/quota/limit";

/// 官方响应体（只取需要的字段，其余忽略）。
#[derive(serde::Deserialize, Debug)]
struct GlmResp {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    data: Option<GlmData>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct GlmData {
    #[serde(default)]
    level: String,
    #[serde(default)]
    limits: Vec<GlmLimit>,
}

#[derive(serde::Deserialize, Debug)]
struct GlmLimit {
    /// "TIME_LIMIT" | "TOKENS_LIMIT"
    #[serde(rename = "type", default)]
    limit_type: String,
    #[serde(default)]
    usage: Option<f64>,
    #[serde(default, rename = "currentValue")]
    current_value: Option<f64>,
    // percentage 缺省为 0，避免 TOKENS_LIMIT 条目因字段缺失被静默丢弃。
    #[serde(default)]
    percentage: f64,
    #[serde(default)]
    unit: Option<i64>,
    #[serde(default)]
    number: Option<i64>,
    #[serde(default, rename = "nextResetTime")]
    next_reset_time: Option<f64>,
}

/// 拉取 GLM 配额。无 key / 失败 / 无数据返回 None。
pub async fn fetch(api_key: &str) -> Option<GlmResult> {
    if api_key.is_empty() {
        return None;
    }

    let resp = match crate::fetcher::http_client()
        .get(GLM_QUOTA_URL)
        .header(ACCEPT, "application/json")
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[GlmFetcher] network error: {e}");
            return None;
        }
    };

    if !resp.status().is_success() {
        eprintln!("[GlmFetcher] HTTP {} (check api_key / rate limit)", resp.status());
        return None;
    }

    let body: GlmResp = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[GlmFetcher] invalid JSON response: {e}");
            return None;
        }
    };

    // code != 200 或无 data → None
    if body.code != 200 {
        eprintln!("[GlmFetcher] unexpected response: code={}", body.code);
        return None;
    }
    let data = body.data?;
    let level = data.level;

    if data.limits.is_empty() {
        eprintln!("[GlmFetcher] no limits in response (Coding Plan not active?)");
        return None;
    }

    let now_ms = now_ms();

    let mut items: Vec<QuotaItem> = Vec::new();
    let mut tool_item: Option<QuotaItem> = None;

    for limit in data.limits {
        let total_val = limit.usage;
        let used_val = limit.current_value;
        // percentage 缺省按 0 处理，不因字段缺失跳过整条。
        let pct = limit.percentage;

        if limit.limit_type == "TIME_LIMIT" {
            // 工具调用次数项：只有 usage>0 才加入。
            if let Some(t) = total_val {
                if t > 0.0 {
                    let used = used_val.unwrap_or(0.0);
                    let reset = fmt_duration(limit.next_reset_time.unwrap_or(0.0) - now_ms);
                    tool_item = Some(QuotaItem {
                        label: "工具调用".into(),
                        used,
                        total: t,
                        unit: "次".into(),
                        detail: if reset.is_empty() {
                            String::new()
                        } else {
                            format!("{reset}后重置")
                        },
                    });
                }
            }
            continue;
        }

        if limit.limit_type == "TOKENS_LIMIT" {
            // TOKENS_LIMIT 始终进入处理；percentage 缺省为 0。
            let label = match (limit.unit.unwrap_or(0), limit.number.unwrap_or(0)) {
                (3, 5) => "5小时限额",
                (6, 1) => "周限额",
                _ => "Token限额",
            };

            // 有绝对值 → tokens 条
            if let Some(t) = total_val {
                if t > 0.0 {
                    let used = used_val.unwrap_or(0.0);
                    items.push(QuotaItem {
                        label: label.into(),
                        used,
                        total: t,
                        unit: "tokens".into(),
                        detail: format!(
                            "已用 {} / {} ({}%)",
                            fmt_tokens(used),
                            fmt_tokens(t),
                            pct as i64
                        ),
                    });
                    continue;
                }
            }
            // 只有百分比 → 百分比条
            let reset = fmt_duration(limit.next_reset_time.unwrap_or(0.0) - now_ms);
            items.push(QuotaItem {
                label: label.into(),
                used: pct,
                total: 100.0,
                unit: "%".into(),
                detail: if reset.is_empty() {
                    String::new()
                } else {
                    format!("{reset}后重置")
                },
            });
        }
    }

    // 工具调用项追加末尾
    if let Some(ti) = tool_item {
        items.push(ti);
    }

    if items.is_empty() {
        return None;
    }

    Some(GlmResult {
        name: "GLM 智谱AI".into(),
        r#type: "quota".into(),
        icon: "🔷".into(),
        color: "#2563EB".into(),
        level: if level.is_empty() { None } else { Some(level) },
        items,
    })
}

/// 当前毫秒时间戳。
fn now_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// 格式化 token 数。
fn fmt_tokens(val: f64) -> String {
    if val >= 100_000_000.0 {
        format!("{:.2}亿", val / 100_000_000.0)
    } else if val >= 10_000.0 {
        format!("{:.1}万", val / 10_000.0)
    } else {
        format!("{}", val as i64)
    }
}

/// 格式化毫秒时长为「X天Y小时 / Y小时Z分 / Z分」。
/// 非正值返回空字符串。
fn fmt_duration(ms: f64) -> String {
    if ms <= 0.0 {
        return String::new();
    }
    let s = ms / 1000.0;
    let d = (s / 86400.0) as i64;
    let h = ((s % 86400.0) / 3600.0) as i64;
    let m = ((s % 3600.0) / 60.0) as i64;
    if d > 0 {
        format!("{d}天{h}小时")
    } else if h > 0 {
        format!("{h}小时{m}分")
    } else {
        format!("{m}分")
    }
}
