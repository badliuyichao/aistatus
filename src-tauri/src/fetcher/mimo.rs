//! 小米 MiMo Token Plan 套餐用量查询。
//!
//! URL:  https://platform.xiaomimimo.com/api/v1/tokenPlan/usage   （用量）
//!       https://platform.xiaomimimo.com/api/v1/tokenPlan/detail  （套餐周期，可选）
//! Auth: 浏览器登录 Cookie（接口不接受 API Key；用户在设置对话框粘贴，
//!       明文存 config.json，见 config::load_mimo_cookie）
//!
//! 非官方公开接口（逆向自控制台 plan-manage 页面），返回三态 MimoFetch：
//!   - NotConfigured：未配置 Cookie
//!   - Failed：网络 / HTTP 失败（除 401/403）
//!   - Ok：正常数据，或业务错误占位条（Cookie 失效给明确提示而非 N/A）
//!
//! 展示规则（对齐官网 plan-manage 页）：
//!   - 条目按 name 映射中文标签：plan_total_token → Token Plan 套餐，
//!     compensation_total_token → 补偿积分；未知 name 原样显示（防丢数据）
//!   - 进度条用 used / limit；额度以 Credit 计，量级常达数十亿，
//!     右侧数值用紧凑格式（B / M / K）避免撑爆卡片
//!   - detail 携带周期重置倒计时（detail 接口的 currentPeriodEnd，
//!     北京时间 "yyyy-MM-dd HH:mm:ss"）；该请求失败只少文案，不影响主数据

use reqwest::header::{ACCEPT, COOKIE};

use crate::data::{MimoFetch, MimoResult, QuotaItem};

const MIMO_USAGE_URL: &str = "https://platform.xiaomimimo.com/api/v1/tokenPlan/usage";
const MIMO_DETAIL_URL: &str = "https://platform.xiaomimimo.com/api/v1/tokenPlan/detail";

/// 官方响应体（只取需要的字段）。
#[derive(serde::Deserialize, Debug, Default)]
struct MimoResp {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    data: Option<MimoData>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct MimoData {
    #[serde(default)]
    usage: Option<MimoUsage>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct MimoUsage {
    #[serde(default)]
    items: Vec<MimoUsageItem>,
}

/// 一个额度条目（套餐或补偿积分）。
#[derive(serde::Deserialize, Debug, Default)]
struct MimoUsageItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    limit: f64,
    #[serde(default)]
    used: f64,
    #[serde(default)]
    percent: f64,
}

/// tokenPlan/detail 响应体（只取周期结束时间）。
#[derive(serde::Deserialize, Debug, Default)]
struct MimoDetailResp {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    data: Option<MimoDetail>,
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct MimoDetail {
    /// 当前计费周期结束（= 额度重置）时间，北京时间 "yyyy-MM-dd HH:mm:ss"。
    #[serde(default)]
    current_period_end: Option<String>,
}

/// 拉取 MiMo Token Plan 用量。
pub async fn fetch(cookie: &str) -> MimoFetch {
    let cookie = cookie.trim();
    if cookie.is_empty() {
        return MimoFetch::NotConfigured;
    }

    let resp = match crate::fetcher::http_client()
        .get(MIMO_USAGE_URL)
        .header(ACCEPT, "application/json")
        .header(COOKIE, cookie)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[MiMoFetcher] network error: {e}");
            return MimoFetch::Failed;
        }
    };

    let status = resp.status();
    if !status.is_success() {
        // 401/403 是登录态问题：Cookie 已失效，给明确提示而非 N/A
        if status.as_u16() == 401 || status.as_u16() == 403 {
            eprintln!("[MiMoFetcher] HTTP {} (cookie expired)", status);
            return MimoFetch::Ok(placeholder_result(
                "Cookie 已失效 · 请在设置中重新粘贴".into(),
            ));
        }
        eprintln!("[MiMoFetcher] HTTP {status}");
        return MimoFetch::Failed;
    }

    let body: MimoResp = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[MiMoFetcher] invalid JSON response: {e}");
            return MimoFetch::Failed;
        }
    };

    if body.code != 0 {
        eprintln!(
            "[MiMoFetcher] API error: code={} message={:?}",
            body.code, body.message
        );
        let msg = match &body.message {
            Some(m) if !m.is_empty() => format!("查询失败 (code {}) · {}", body.code, m),
            _ => format!("查询失败 (code {}) · Cookie 可能已失效", body.code),
        };
        return MimoFetch::Ok(placeholder_result(msg));
    }

    let items = body
        .data
        .and_then(|d| d.usage)
        .map(|u| u.items)
        .unwrap_or_default();
    if items.is_empty() {
        eprintln!("[MiMoFetcher] no usage items in response (plan not active?)");
        return MimoFetch::Ok(placeholder_result("暂无套餐数据".into()));
    }

    // 周期重置倒计时：detail 接口拿 currentPeriodEnd。独立于主数据，
    // 任何失败只少一段文案（返回 None），不影响卡片。
    let reset_hint = fetch_reset_hint(cookie).await;

    MimoFetch::Ok(MimoResult {
        name: "MiMo".into(),
        r#type: "quota".into(),
        icon: "🧡".into(),
        color: "#FF6900".into(),
        items: items
            .iter()
            .map(|it| to_quota_item(it, reset_hint.as_deref()))
            .collect(),
    })
}

/// 拉套餐周期结束时间 → 「N天后重置」文案；任何失败返回 None。
async fn fetch_reset_hint(cookie: &str) -> Option<String> {
    let resp = crate::fetcher::http_client()
        .get(MIMO_DETAIL_URL)
        .header(ACCEPT, "application/json")
        .header(COOKIE, cookie)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: MimoDetailResp = resp.json().await.ok()?;
    if body.code != 0 {
        return None;
    }
    let end = body.data?.current_period_end?;
    let days = days_until_period_end(&end, now_epoch_secs())?;
    match days {
        0 => Some("今日重置".into()),
        d if d > 0 => Some(format!("{d}天后重置")),
        _ => None, // 已过期（周期切换瞬间），不显示
    }
}

/// 一个官方 usage 条目 → 展示条目。`reset_hint` 仅拼进 Token Plan 套餐条
/// （补偿积分不随套餐周期重置，不附带）。
fn to_quota_item(item: &MimoUsageItem, reset_hint: Option<&str>) -> QuotaItem {
    let label = match item.name.as_str() {
        "plan_total_token" => "Token Plan 套餐",
        "compensation_total_token" => "补偿积分",
        other => other,
    };
    QuotaItem {
        label: label.into(),
        display_value: Some(format!(
            "{} / {}",
            compact(item.used),
            compact(item.limit)
        )),
        used: item.used,
        total: item.limit,
        unit: "Credits".into(),
        // API 的 percent 是 0–1 小数（实测 used/limit=37.1% 时 percent=0.37），
        // 展示需 ×100；数值本身冗余（used/limit 可算），仅作 detail 文案。
        detail: match reset_hint {
            Some(h) if item.name == "plan_total_token" && !h.is_empty() => {
                format!("已用 {:.1}% · {h}", item.percent * 100.0)
            }
            _ => format!("已用 {:.1}%", item.percent * 100.0),
        },
    }
}

/// 当前 Unix 秒，折算到北京时区（+8h）：平台返回的时间是北京时间，
/// 与用户本地时区无关地按北京日历日计数。
fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + 8 * 3600
}

/// "yyyy-MM-dd HH:mm:ss"（北京时间）距 `now_epoch`（已含 +8h）的整日差。
/// 同日返回 0，前一日返回 -1；解析失败返回 None。
fn days_until_period_end(s: &str, now_epoch: i64) -> Option<i64> {
    let (y, m, d) = parse_civil_date(s)?;
    let now_day = now_epoch.div_euclid(86400);
    Some(days_from_civil(y, m, d) - now_day)
}

/// 取 "yyyy-MM-dd ..." 前三段年月日（分隔符兼容 `-` / `T` / 空格）。
fn parse_civil_date(s: &str) -> Option<(i64, i64, i64)> {
    let mut parts = s.split(&['-', 'T', ' '][..]);
    let y = parts.next()?.parse().ok()?;
    let m = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Some((y, m, d))
}

/// 公历日期 → 自 Unix 纪元的天数（Howard Hinnant 的 days_from_civil，
/// 纯整数运算，不引入 chrono 依赖）。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// 紧凑格式化大数：5000000000 → "5B"，1234567 → "1.2M"，999 → "999"。
/// 尾数 ".0" 去掉（"5.0B" → "5B"）。
fn compact(n: f64) -> String {
    let (v, suffix) = if n.abs() >= 1e9 {
        (n / 1e9, "B")
    } else if n.abs() >= 1e6 {
        (n / 1e6, "M")
    } else if n.abs() >= 1e3 {
        (n / 1e3, "K")
    } else {
        return format!("{}", n.round() as i64);
    };
    let s = format!("{v:.1}");
    let s = match s.strip_suffix(".0") {
        Some(stripped) => stripped.to_string(),
        None => s,
    };
    format!("{s}{suffix}")
}

/// 占位结果（Cookie 失效 / 业务错误 / 无数据时）。
fn placeholder_result(msg: String) -> MimoResult {
    MimoResult {
        name: "MiMo".into(),
        r#type: "quota".into(),
        icon: "🧡".into(),
        color: "#FF6900".into(),
        items: vec![QuotaItem {
            label: "Token Plan 套餐".into(),
            display_value: None,
            used: 0.0,
            total: 100.0,
            unit: "Credits".into(),
            detail: msg,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_formats_large_numbers() {
        assert_eq!(compact(5_000_000_000.0), "5B");
        assert_eq!(compact(1_234_567_890.0), "1.2B");
        assert_eq!(compact(1_234_567.0), "1.2M");
        assert_eq!(compact(100_000.0), "100K");
        assert_eq!(compact(999.0), "999");
        assert_eq!(compact(0.0), "0");
    }

    #[test]
    fn parses_usage_items_with_name_mapping() {
        let resp: MimoResp = serde_json::from_str(
            r#"{
                "code": 0,
                "message": "success",
                "data": { "usage": { "items": [
                    { "name": "plan_total_token", "limit": 5000000000, "used": 1234567890, "percent": 0.247 },
                    { "name": "compensation_total_token", "limit": 100000, "used": 0, "percent": 0.0 },
                    { "name": "future_new_type", "limit": 50, "used": 10, "percent": 0.2 }
                ]}}
            }"#,
        )
        .unwrap();
        assert_eq!(resp.code, 0);
        let items = resp.data.unwrap().usage.unwrap().items;
        assert_eq!(items.len(), 3);

        let q = to_quota_item(&items[0], None);
        assert_eq!(q.label, "Token Plan 套餐");
        assert_eq!(q.display_value.as_deref(), Some("1.2B / 5B"));
        assert_eq!(q.used, 1234567890.0);
        assert_eq!(q.total, 5000000000.0);
        assert_eq!(q.detail, "已用 24.7%");

        assert_eq!(to_quota_item(&items[1], None).label, "补偿积分");
        // 未知条目按原 name 显示，不丢数据
        assert_eq!(to_quota_item(&items[2], None).label, "future_new_type");
    }

    #[test]
    fn quota_item_appends_reset_hint_for_plan_only() {
        let plan = MimoUsageItem {
            name: "plan_total_token".into(),
            limit: 100.0,
            used: 38.0,
            percent: 0.38,
        };
        let comp = MimoUsageItem {
            name: "compensation_total_token".into(),
            limit: 0.0,
            used: 0.0,
            percent: 0.0,
        };
        // 套餐条附带倒计时；补偿积分不带
        assert_eq!(
            to_quota_item(&plan, Some("29天后重置")).detail,
            "已用 38.0% · 29天后重置"
        );
        assert_eq!(to_quota_item(&comp, Some("29天后重置")).detail, "已用 0.0%");
        // 空文案视为无
        assert_eq!(to_quota_item(&plan, Some("")).detail, "已用 38.0%");
    }

    #[test]
    fn parses_detail_current_period_end() {
        let resp: MimoDetailResp = serde_json::from_str(
            r#"{ "code": 0, "data": { "planCode": "lite", "planName": "Lite",
                "currentPeriodEnd": "2026-09-24 23:59:59", "expired": false } }"#,
        )
        .unwrap();
        assert_eq!(resp.code, 0);
        assert_eq!(
            resp.data.unwrap().current_period_end.as_deref(),
            Some("2026-09-24 23:59:59")
        );
    }

    #[test]
    fn days_until_counts_calendar_days() {
        // 北京时间 2026-08-26 17:30 → 已含 +8h 的 epoch
        let now = days_from_civil(2026, 8, 26) * 86400 + 17 * 3600 + 30 * 60;
        assert_eq!(
            days_until_period_end("2026-09-24 23:59:59", now),
            Some(29),
            "8/26 → 9/24 跨 8 月末，共 29 天"
        );
        assert_eq!(days_until_period_end("2026-08-26 12:00:00", now), Some(0));
        assert_eq!(days_until_period_end("2026-08-25 12:00:00", now), Some(-1));
        assert_eq!(days_until_period_end("not-a-date", now), None);
        // 闰年边界：2028-02-28 → 2028-03-01 应为 2 天
        let leap_now = days_from_civil(2028, 2, 28) * 86400;
        assert_eq!(days_until_period_end("2028-03-01 00:00:00", leap_now), Some(2));
    }

    #[test]
    fn parses_api_error_code() {
        let resp: MimoResp =
            serde_json::from_str(r#"{ "code": 401, "message": "unauthorized" }"#).unwrap();
        assert_eq!(resp.code, 401);
        assert_eq!(resp.message.as_deref(), Some("unauthorized"));
    }

    #[test]
    fn parses_empty_items() {
        let resp: MimoResp =
            serde_json::from_str(r#"{ "code": 0, "data": { "usage": { "items": [] } } }"#)
                .unwrap();
        assert!(resp.data.unwrap().usage.unwrap().items.is_empty());
    }
}
