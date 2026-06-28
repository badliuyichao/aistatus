//! MiniMax 稀宇科技 Coding Plan 用量查询 —— 对照 legacy `minimax_fetcher.py`。
//!
//! URL:    https://www.minimaxi.com/v1/api/openplatform/coding_plan/remains
//! Auth:   Bearer <api_key>
//! 返回配额型（type: quota）。
//!
//! 核心是 `_aggregate_window` 聚合逻辑：跨所有活跃 model 求平均 used%。
//!   - status == 1 且有 remaining_percent → 活跃，纳入聚合
//!   - used% = 100 - remaining%（对活跃 model 求平均）
//!   - 重置时间取最早（min remains_time）
//!   - 无活跃 model → 「inactive」条（5h 用默认「本周期未开始或无用量」，
//!     周限额用「无限额」满条）

use reqwest::header::{ACCEPT, AUTHORIZATION};

use crate::data::{MinimaxResult, QuotaItem};

const MINIMAX_QUOTA_URL: &str =
    "https://www.minimaxi.com/v1/api/openplatform/coding_plan/remains";

/// 官方响应体（只取需要的字段）。
#[derive(serde::Deserialize, Debug, Default)]
struct MinimaxResp {
    #[serde(default)]
    base_resp: Option<BaseResp>,
    // 用 Option + default 容错 API 返回 "model_remains": null
    // （对齐 legacy Python `body.get("model_remains") or []`）。否则 serde 对
    // null 反序列化 Vec 会失败，整个响应被静默吞掉，导致 MiniMax 整卡消失。
    #[serde(default)]
    model_remains: Option<Vec<ModelRemain>>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct BaseResp {
    #[serde(default)]
    status_code: Option<i64>,
}

/// 一个 model 在两个窗口（5h / 周）的用量数据。
/// 字段都 optional，缺失视为该 model 不参与对应窗口。
#[derive(serde::Deserialize, Debug)]
struct ModelRemain {
    #[serde(default)]
    model_name: Option<String>,
    // 5h 窗口
    #[serde(default)]
    current_interval_status: Option<i64>,
    #[serde(default)]
    current_interval_remaining_percent: Option<f64>,
    #[serde(default)]
    remains_time: Option<f64>,
    // 周窗口
    #[serde(default)]
    current_weekly_status: Option<i64>,
    #[serde(default)]
    current_weekly_remaining_percent: Option<f64>,
    #[serde(default)]
    weekly_remains_time: Option<f64>,
}

/// 标识取哪个窗口的字段（对应 Python `status_field`/`pct_field`/`remains_field` 字符串）。
enum Window {
    Interval,
    Weekly,
}

impl Window {
    fn status(&self, m: &ModelRemain) -> Option<i64> {
        match self {
            Window::Interval => m.current_interval_status,
            Window::Weekly => m.current_weekly_status,
        }
    }
    fn remaining_pct(&self, m: &ModelRemain) -> Option<f64> {
        match self {
            Window::Interval => m.current_interval_remaining_percent,
            Window::Weekly => m.current_weekly_remaining_percent,
        }
    }
    fn remains_time(&self, m: &ModelRemain) -> Option<f64> {
        match self {
            Window::Interval => m.remains_time,
            Window::Weekly => m.weekly_remains_time,
        }
    }
}

/// 拉取 MiniMax 配额。无 key/失败返回 None；
/// API 错误或无数据返回带占位条的 MinimaxResult（对应 Python 逻辑）。
pub async fn fetch(api_key: &str) -> Option<MinimaxResult> {
    if api_key.is_empty() {
        return None;
    }

    let resp = match crate::fetcher::http_client()
        .get(MINIMAX_QUOTA_URL)
        .header(ACCEPT, "application/json")
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[MiniMaxFetcher] network error: {e}");
            return None;
        }
    };

    if !resp.status().is_success() {
        eprintln!("[MiniMaxFetcher] HTTP {} (check api_key / rate limit)", resp.status());
        return None;
    }

    let body: MinimaxResp = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[MiniMaxFetcher] invalid JSON response: {e}");
            return None;
        }
    };

    // base_resp.status_code 非 0/None → API 错误占位
    if let Some(br) = &body.base_resp {
        if let Some(code) = br.status_code {
            if code != 0 {
                eprintln!("[MiniMaxFetcher] API error in base_resp: status_code={code}");
                return Some(MinimaxResult {
                    name: "MiniMax".into(),
                    r#type: "quota".into(),
                    icon: "🟠".into(),
                    color: "#EA580C".into(),
                    items: placeholder_items(format!("API 错误 {code}")),
                });
            }
        }
    }

    let remains = body.model_remains.as_deref().unwrap_or(&[]);
    if remains.is_empty() {
        eprintln!("[MiniMaxFetcher] no model_remains in response (plan not active?)");
        return Some(MinimaxResult {
            name: "MiniMax".into(),
            r#type: "quota".into(),
            icon: "🟠".into(),
            color: "#EA580C".into(),
            items: placeholder_items("暂无用量数据".into()),
        });
    }

    let models = remains;

    // 两个窗口聚合
    let interval = aggregate_window(
        models,
        Window::Interval,
        "5小时限额",
        0.0,
        "本周期未开始或无用量",
    );
    let weekly = aggregate_window(
        models,
        Window::Weekly,
        "周限额",
        100.0, // 周窗口无活跃 → 「无限额」满条
        "无限额",
    );

    Some(MinimaxResult {
        name: "MiniMax".into(),
        r#type: "quota".into(),
        icon: "🟠".into(),
        color: "#EA580C".into(),
        items: vec![interval, weekly],
    })
}

/// 聚合一个窗口：跨所有活跃 model 求平均 used%。
/// 对应 Python `_aggregate_window`。
fn aggregate_window(
    models: &[ModelRemain],
    window: Window,
    label: &str,
    inactive_used: f64,
    inactive_detail: &str,
) -> QuotaItem {
    // 活跃 = status==1 且 remaining_percent 是数字
    let active: Vec<&ModelRemain> = models
        .iter()
        .filter(|m| window.status(m) == Some(1) && window.remaining_pct(m).is_some())
        .collect();

    if active.is_empty() {
        return QuotaItem {
            label: label.into(),
            used: inactive_used,
            total: 100.0,
            unit: "%".into(),
            detail: inactive_detail.into(),
        };
    }

    // 平均 used% = mean(100 - remaining%)
    let used_pcts: Vec<f64> = active
        .iter()
        .map(|m| 100.0 - window.remaining_pct(m).unwrap_or(100.0))
        .collect();
    let used_pct = used_pcts.iter().sum::<f64>() / used_pcts.len() as f64;

    // 重置时间取最早（min remains_time）
    let reset_dur = {
        let rems: Vec<f64> = active
            .iter()
            .filter_map(|m| window.remains_time(m))
            .collect();
        if rems.is_empty() {
            String::new()
        } else {
            fmt_duration(rems.iter().cloned().fold(f64::INFINITY, f64::min))
        }
    };

    // model 名提示（去 general）
    let mut model_names: Vec<String> = active
        .iter()
        .filter_map(|m| m.model_name.clone())
        .filter(|n| !n.is_empty() && n.to_lowercase() != "general")
        .collect();
    model_names.sort();
    model_names.dedup();
    let model_hint = if model_names.is_empty() {
        String::new()
    } else {
        format!(" ({})", model_names.join(", "))
    };

    let detail = if reset_dur.is_empty() {
        String::new()
    } else {
        format!("{reset_dur}后重置{model_hint}")
    };

    QuotaItem {
        label: label.into(),
        used: (used_pct * 10.0).round() / 10.0, // round(used_pct, 1)
        total: 100.0,
        unit: "%".into(),
        detail,
    }
}

/// 占位条（API 错误 / 无数据时），对应 Python `_placeholder_items`。
fn placeholder_items(msg: String) -> Vec<QuotaItem> {
    vec![
        QuotaItem {
            label: "5小时限额".into(),
            used: 0.0,
            total: 100.0,
            unit: "%".into(),
            detail: msg.clone(),
        },
        QuotaItem {
            label: "周限额".into(),
            used: 0.0,
            total: 100.0,
            unit: "%".into(),
            detail: msg,
        },
    ]
}

/// 格式化毫秒时长，对应 Python `_fmt_duration`（与 glm 模块同名同实现）。
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
