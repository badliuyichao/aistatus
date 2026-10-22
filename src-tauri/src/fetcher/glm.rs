//! GLM 智谱AI Coding Plan 用量查询。
//!
//! URL:    https://open.bigmodel.cn/api/monitor/usage/quota/limit
//! Auth:   Bearer <api_key>（用户在设置对话框运行期配置，见 config::PlanConfig）
//! 返回配额型（type: quota），带 level 套餐档位。
//!
//! 关键解析逻辑：
//!   - body.code != 200 或无 data → Failed
//!   - data.limits 为空 → Failed
//!   - limit.type == "TIME_LIMIT" 且 usage>0 → 「工具调用」次条（绝对值，追加末尾）
//!   - limit.type == "TOKENS_LIMIT"：
//!       unit==3,num==5 → 「5小时限额」
//!       unit==6,num==1 → 「周限额」
//!       否则          → 「Token限额」
//!     有 usage 绝对值 → tokens 条；否则 → 百分比条
//!   - level 非空 → 卡片标题显示「GLM 智谱AI · {LEVEL 大写}」
//!
//! 错误语义（PlanFetch 三态）：HTTP 401/403 → Ok(占位「API Key 无效」，
//! key 为用户粘贴、拼错可诊断）；其余网络 / HTTP / 解析失败 → Failed（卡片 N/A）。

use reqwest::header::{ACCEPT, AUTHORIZATION};

use crate::data::{PlanFetch, QuotaItem, ServiceInfo};

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

/// 拉取 GLM 配额。空 key → Skipped；失败 → Failed；成功 / 占位 → Ok。
pub async fn fetch(api_key: &str) -> PlanFetch {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return PlanFetch::Skipped;
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
            return PlanFetch::Failed;
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        // 401/403：认证失败（key 无效 / 被删）。key 为用户运行期配置，
        // 给明确占位提示而非 N/A（对齐 DeepSeek / MiMo 的处理思路）。
        if status.as_u16() == 401 || status.as_u16() == 403 {
            eprintln!("[GlmFetcher] HTTP {status} (invalid api key)");
            return PlanFetch::Ok(placeholder_service("API Key 无效 · 请在设置中检查".into()));
        }
        eprintln!("[GlmFetcher] HTTP {status} (check api_key / rate limit)");
        return PlanFetch::Failed;
    }

    let body: GlmResp = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[GlmFetcher] invalid JSON response: {e}");
            return PlanFetch::Failed;
        }
    };

    // code != 200：GLM 的认证失败走 HTTP 200 + 业务码（如 code=401），
    // 不产生非 2xx 状态——按业务码映射占位，key 为用户配置必须可诊断。
    if body.code != 200 {
        eprintln!("[GlmFetcher] unexpected response: code={}", body.code);
        return PlanFetch::Ok(match body.code {
            401 | 403 => placeholder_service("API Key 无效 · 请在设置中检查".into()),
            code => placeholder_service(format!(
                "接口返回异常 (code {code}) · 请稍后重试或在设置中检查 Key"
            )),
        });
    }
    let Some(data) = body.data else {
        // 认证通过（HTTP 200 + code 200）但没有数据：多为该账号未开通 Coding Plan。
        // 给明确占位而非 N/A——key 为用户配置，必须可诊断（对齐 MiMo/DeepSeek 的处理）。
        eprintln!("[GlmFetcher] 200 but no data in response");
        return PlanFetch::Ok(placeholder_service(
            "未查询到 Coding Plan 用量 · 请确认该账号已开通".into(),
        ));
    };
    let level = data.level;

    if data.limits.is_empty() {
        eprintln!("[GlmFetcher] no limits in response (Coding Plan not active?)");
        return PlanFetch::Ok(placeholder_service(
            "未查询到 Coding Plan 用量 · 请确认该账号已开通".into(),
        ));
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
                        display_value: None,
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

        if limit.limit_type == "TOKENS_LIMIT" || limit.limit_type == "CREDIT_LIMIT" {
            // 新版积分制 Coding Plan（level lite 等）返回 CREDIT_LIMIT，
            // 字段语义与 TOKENS_LIMIT 一致（usage=总量、currentValue=已用、
            // unit/number 标识窗口），仅单位不同——同一条分支解析。
            // 两者都始终进入处理；percentage 缺省为 0。
            let is_credit = limit.limit_type == "CREDIT_LIMIT";
            let label = match (limit.unit.unwrap_or(0), limit.number.unwrap_or(0)) {
                (3, 5) => "5小时限额",
                (6, 1) => "周限额",
                _ => {
                    if is_credit {
                        "积分限额"
                    } else {
                        "Token限额"
                    }
                }
            };
            let unit_name = if is_credit { "积分" } else { "tokens" };

            // 有绝对值 → 数量条（tokens / 积分）
            if let Some(t) = total_val {
                if t > 0.0 {
                    let used = used_val.unwrap_or(0.0);
                    let reset = fmt_duration(limit.next_reset_time.unwrap_or(0.0) - now_ms);
                    items.push(QuotaItem {
                        label: label.into(),
                        display_value: None,
                        used,
                        total: t,
                        unit: unit_name.into(),
                        // 数量条也要带重置倒计时（与百分比条 / MiMo 的
                        // 「已用 … · N天后重置」一致），否则积分制实例
                        // 只有用量、看不出窗口何时刷新。
                        detail: amount_detail(used, t, pct, &reset),
                    });
                    continue;
                }
            }
            // 只有百分比 → 百分比条
            let reset = fmt_duration(limit.next_reset_time.unwrap_or(0.0) - now_ms);
            items.push(QuotaItem {
                label: label.into(),
                display_value: None,
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
        eprintln!("[GlmFetcher] limits parsed to 0 usable items");
        return PlanFetch::Ok(placeholder_service(
            "未查询到 Coding Plan 用量 · 请确认该账号已开通".into(),
        ));
    }

    // 标题后缀：level 非空 → 「GLM 智谱AI · {LEVEL 大写}」
    let name = match level.as_str() {
        "" => "GLM 智谱AI".to_string(),
        lv => format!("GLM 智谱AI · {}", lv.to_uppercase()),
    };

    PlanFetch::Ok(ServiceInfo {
        name,
        r#type: "quota".into(),
        vendor: "glm".into(),
        icon: "🔷".into(),
        color: "#2563EB".into(),
        items,
        ..Default::default()
    })
}

/// 401/403 占位卡片：两条配额条 detail 带原因，保证提示可见。
fn placeholder_service(msg: String) -> ServiceInfo {
    let item = |label: &str| QuotaItem {
        label: label.into(),
        display_value: None,
        used: 0.0,
        total: 100.0,
        unit: "%".into(),
        detail: msg.clone(),
    };
    ServiceInfo {
        name: "GLM 智谱AI".into(),
        r#type: "quota".into(),
        vendor: "glm".into(),
        icon: "🔷".into(),
        color: "#2563EB".into(),
        items: vec![item("5小时限额"), item("周限额")],
        placeholder: true,
        ..Default::default()
    }
}

/// 当前毫秒时间戳。
fn now_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

/// 数量条详情：已用量 + 可选重置倒计时。reset 为空（无 nextResetTime /
/// 已过期）时只显示用量，避免留下悬空的「后重置」。
fn amount_detail(used: f64, total: f64, pct: f64, reset: &str) -> String {
    let usage = format!(
        "已用 {} / {} ({}%)",
        fmt_tokens(used),
        fmt_tokens(total),
        pct as i64
    );
    if reset.is_empty() {
        usage
    } else {
        format!("{usage} · {reset}后重置")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amount_detail_appends_reset_hint() {
        // 积分制（CREDIT_LIMIT）数量条：用量 + 重置倒计时
        assert_eq!(
            amount_detail(2001.0, 2000.0, 100.0, "1小时11分"),
            "已用 2001 / 2000 (100%) · 1小时11分后重置"
        );
        // 无 nextResetTime（fmt_duration 返回空）时只留用量，不留悬空「后重置」
        assert_eq!(
            amount_detail(6731.0, 10000.0, 67.0, ""),
            "已用 6731 / 1.0万 (67%)"
        );
    }

    #[test]
    fn fmt_duration_picks_largest_units() {
        assert_eq!(
            fmt_duration(5.0 * 3600.0 * 1000.0 + 21.0 * 60.0 * 1000.0),
            "5小时21分"
        );
        assert_eq!(fmt_duration(26.0 * 86400.0 * 1000.0), "26天0小时");
        assert_eq!(fmt_duration(0.0), "");
        assert_eq!(fmt_duration(-1000.0), "");
    }
}
