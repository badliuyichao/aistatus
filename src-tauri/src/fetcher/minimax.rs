//! MiniMax 稀宇科技 Coding Plan 用量查询。
//!
//! URL:    https://www.minimaxi.com/v1/api/openplatform/coding_plan/remains
//! Auth:   Bearer <api_key>
//! 返回配额型（type: quota）。
//!
//! 展示规则（对齐 MiniMax 官网）：
//!   - 活跃窗口（有 status==1 的 model）：显示**已用%** = 100 - 平均 remaining_percent，
//!     进度条按已用填充（满条=用尽）。官网此时也显示已用百分比（用了 2% 显示 2%）。
//!   - status==2：额度已用尽，remaining_percent 通常为 0；5h 提示「额度已用尽」。
//!   - status==3：窗口未计费/不限，remaining 不参与已用比例；周限额显示 `∞`。
//!   - 重置时间取最早（min remains_time，含 inactive model）。
//!
//! ⚠️ 5h inactive 的 100% 是「满额可用」占位（不是「100% 已用」），与活跃
//!    分支的已用%语义不同；weekly inactive 用 `display_value=∞` 明确表达无限额。

use reqwest::header::{ACCEPT, AUTHORIZATION};

use crate::data::{MinimaxResult, QuotaItem};

const MINIMAX_QUOTA_URL: &str = "https://www.minimaxi.com/v1/api/openplatform/coding_plan/remains";

/// 官方响应体（只取需要的字段）。
#[derive(serde::Deserialize, Debug, Default)]
struct MinimaxResp {
    #[serde(default)]
    base_resp: Option<BaseResp>,
    // 用 Option + default 容错 API 返回 "model_remains": null；否则 serde
    // 将 null 反序列化为 Vec 时会失败，整个响应被静默吞掉，导致 MiniMax 整卡消失。
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

/// 标识取哪个窗口的字段。
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
/// API 错误或无数据返回带占位条的 MinimaxResult。
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
        eprintln!(
            "[MiniMaxFetcher] HTTP {} (check api_key / rate limit)",
            resp.status()
        );
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

    // 两个窗口聚合：受限窗口展示已用%，无限周限额用 ∞。
    let interval = aggregate_window(
        models,
        Window::Interval,
        "5小时限额",
        "本周期可用",
        true,
        None,
    );
    let weekly = aggregate_window(models, Window::Weekly, "周限额", "无限额", false, Some("∞"));

    Some(MinimaxResult {
        name: "MiniMax".into(),
        r#type: "quota".into(),
        icon: "🟠".into(),
        color: "#EA580C".into(),
        items: vec![interval, weekly],
    })
}

/// 聚合一个窗口 → 展示对齐 MiniMax 官网的百分比。
///
/// - 活跃窗口（有 status==1 的 model）：显示**已用%** = 100 - 平均 remaining_percent，
///   进度条按已用填充（满条=用尽）。官网此时也显示已用百分比（用了 2% 显示 2%）。
/// - 无 status==1、但有 status==2：额度已用尽，显示「额度已用尽 · …后重置」。
/// - 仅 status==3：窗口未计费/不限；5h 保留满额可用占位，weekly 显示 `∞`。
///
/// `show_inactive_reset`：无活跃 model 时是否在 detail 后附重置倒计时。
/// 5h「本周期可用」需要（窗口会刷新），weekly「无限额」不需要（谈不上重置）。
/// `inactive_display_value`：仅 status==3 时覆盖右侧数值（weekly 用 `∞`）。
fn aggregate_window(
    models: &[ModelRemain],
    window: Window,
    label: &str,
    inactive_detail: &str,
    show_inactive_reset: bool,
    inactive_display_value: Option<&str>,
) -> QuotaItem {
    // status==1：正常使用中；status==2：额度已用尽；status==3：未计费/不限。
    let active: Vec<&ModelRemain> = models
        .iter()
        .filter(|m| window.status(m) == Some(1) && window.remaining_pct(m).is_some())
        .collect();
    let exhausted: Vec<&ModelRemain> = models
        .iter()
        .filter(|m| window.status(m) == Some(2) && window.remaining_pct(m).is_some())
        .collect();

    // 有活跃 model 时，将同窗口内已用尽的 model 一并计入平均；只有已用尽 model
    // 时直接按其 remaining 计算。仅 status==3 才走 100% 满额可用占位。
    let metered: Vec<&ModelRemain> = active.iter().chain(exhausted.iter()).copied().collect();
    let used_pct = if metered.is_empty() {
        100.0
    } else {
        let remaining: Vec<f64> = metered
            .iter()
            .map(|m| window.remaining_pct(m).unwrap_or(100.0))
            .collect();
        let mean_remaining = remaining.iter().sum::<f64>() / remaining.len() as f64;
        100.0 - mean_remaining
    };

    // 重置时间：取所有 model 中最早的 remains_time（对 inactive 也有意义）
    let reset_dur = {
        let rems: Vec<f64> = models
            .iter()
            .filter_map(|m| window.remains_time(m))
            .collect();
        if rems.is_empty() {
            String::new()
        } else {
            fmt_duration(rems.iter().cloned().fold(f64::INFINITY, f64::min))
        }
    };

    // model 名提示（去 general；仅活跃 model）
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

    let detail = if active.is_empty() && !exhausted.is_empty() {
        if reset_dur.is_empty() {
            "额度已用尽".into()
        } else {
            format!("额度已用尽 · {reset_dur}后重置")
        }
    } else if metered.is_empty() {
        // 仅 status==3：满额可用或不限额。按需附重置倒计时。
        if show_inactive_reset && !reset_dur.is_empty() {
            format!("{inactive_detail} · {reset_dur}后重置")
        } else {
            inactive_detail.into()
        }
    } else if reset_dur.is_empty() {
        String::new()
    } else {
        format!("{reset_dur}后重置{model_hint}")
    };

    QuotaItem {
        label: label.into(),
        display_value: if metered.is_empty() {
            inactive_display_value.map(str::to_string)
        } else {
            None
        },
        used: (used_pct * 10.0).round() / 10.0, // round(used_pct, 1)
        total: 100.0,
        unit: "%".into(),
        detail,
    }
}

/// 占位条（API 错误 / 无数据时）。
fn placeholder_items(msg: String) -> Vec<QuotaItem> {
    vec![
        QuotaItem {
            label: "5小时限额".into(),
            display_value: None,
            used: 0.0,
            total: 100.0,
            unit: "%".into(),
            detail: msg.clone(),
        },
        QuotaItem {
            label: "周限额".into(),
            display_value: None,
            used: 0.0,
            total: 100.0,
            unit: "%".into(),
            detail: msg,
        },
    ]
}

/// 格式化毫秒时长。
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

    fn model(
        name: &str,
        interval_status: i64,
        interval_remaining: f64,
        interval_time: f64,
        weekly_status: i64,
        weekly_remaining: f64,
    ) -> ModelRemain {
        ModelRemain {
            model_name: Some(name.into()),
            current_interval_status: Some(interval_status),
            current_interval_remaining_percent: Some(interval_remaining),
            remains_time: Some(interval_time),
            current_weekly_status: Some(weekly_status),
            current_weekly_remaining_percent: Some(weekly_remaining),
            weekly_remains_time: None,
        }
    }

    #[test]
    fn exhausted_interval_uses_exhausted_message() {
        let models = vec![
            model("general", 2, 0.0, 13_236_972.0, 3, 100.0),
            model("video", 3, 100.0, 27_636_972.0, 3, 100.0),
        ];

        let item = aggregate_window(
            &models,
            Window::Interval,
            "5小时限额",
            "本周期可用",
            true,
            None,
        );

        assert_eq!(item.used, 100.0);
        assert_eq!(item.display_value, None);
        assert_eq!(item.detail, "额度已用尽 · 3小时40分后重置");
    }

    #[test]
    fn unlimited_weekly_uses_infinity_display() {
        let models = vec![
            model("general", 2, 0.0, 13_236_972.0, 3, 100.0),
            model("video", 3, 100.0, 27_636_972.0, 3, 100.0),
        ];

        let item = aggregate_window(
            &models,
            Window::Weekly,
            "周限额",
            "无限额",
            false,
            Some("∞"),
        );

        assert_eq!(item.display_value.as_deref(), Some("∞"));
        assert_eq!(item.detail, "无限额");
    }

    #[test]
    fn active_weekly_keeps_numeric_display() {
        let models = vec![model("general", 1, 80.0, 13_236_972.0, 1, 75.0)];

        let item = aggregate_window(
            &models,
            Window::Weekly,
            "周限额",
            "无限额",
            false,
            Some("∞"),
        );

        assert_eq!(item.used, 25.0);
        assert_eq!(item.display_value, None);
    }
}
