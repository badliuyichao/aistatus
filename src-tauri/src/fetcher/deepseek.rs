//! DeepSeek 余额查询。
//!
//! URL:    https://api.deepseek.com/user/balance
//! Auth:   Bearer <api_key>
//! 返回余额型 ServiceInfo。
//!
//! 关键逻辑：
//!   - HTTP 4xx/5xx / 网络错 / JSON 错 → None
//!   - `is_available == false` → 红色占位「账户不可用或额度耗尽」
//!   - 读 `balance_infos[0].total_balance`；货币默认 ¥
//!   - 余额 > 10 → 蓝色 #4D6BFE，否则 → 红色 #FF5252

use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::Deserializer;

use crate::data::DeepSeekResult;

const DEEPSEEK_BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

/// 官方响应体（只取需要的字段）。
#[derive(serde::Deserialize, Debug)]
struct DeepSeekResp {
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(serde::Deserialize, Debug)]
struct BalanceInfo {
    // DeepSeek 官方 API 返回的 total_balance 是【字符串】（如 "65.02"），
    // 而非 JSON 数字。用 deserialize_f64 兼容字符串与数字两种格式，
    // 兼容 API 以字符串或数字返回余额。
    #[serde(default, deserialize_with = "deserialize_f64")]
    total_balance: f64,
    #[serde(default)]
    currency: String,
}

/// 反序列化 f64，兼容 JSON 数字与字符串（如 "65.02" / 65.02）。
/// 解析失败或缺失时返回 0.0。
fn deserialize_f64<'de, D: Deserializer<'de>>(de: D) -> Result<f64, D::Error> {
    use serde::Deserialize;
    let v = serde_json::Value::deserialize(de)?;
    Ok(match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        serde_json::Value::String(s) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    })
}

/// 拉取 DeepSeek 余额。无 key 或失败返回 None。
pub async fn fetch(api_key: &str) -> Option<DeepSeekResult> {
    if api_key.is_empty() {
        return None;
    }

    let resp = match crate::fetcher::http_client()
        .get(DEEPSEEK_BALANCE_URL)
        .header(ACCEPT, "application/json")
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[DeepSeekFetcher] network error: {e}");
            return None;
        }
    };

    // 4xx/5xx → None。
    if !resp.status().is_success() {
        eprintln!("[DeepSeekFetcher] HTTP {} (check api_key / rate limit)", resp.status());
        return None;
    }

    let body: DeepSeekResp = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[DeepSeekFetcher] invalid JSON response: {e}");
            return None;
        }
    };

    // 账户不可用 → 红色占位。
    if !body.is_available {
        eprintln!("[DeepSeekFetcher] account unavailable or balance exhausted");
        return Some(DeepSeekResult {
            name: "DeepSeek".into(),
            r#type: "balance".into(),
            icon: "🟢".into(),
            balance: 0.0,
            currency: "¥".into(),
            unit: "元".into(),
            detail: "账户不可用或额度耗尽".into(),
            color: "#FF5252".into(),
        });
    }

    // 读余额信息
    let (total, currency_raw) = match body.balance_infos.first() {
        Some(info) => (info.total_balance, info.currency.clone()),
        None => {
            eprintln!("[DeepSeekFetcher] no balance_infos in response (unexpected shape)");
            (0.0, String::new())
        }
    };

    // 货币归一：CNY/¥ → ¥，其余原样
    let currency = if currency_raw == "CNY" || currency_raw == "¥" {
        "¥"
    } else if currency_raw.is_empty() {
        "¥"
    } else {
        &currency_raw
    }
    .to_string();

    let color = if total > 10.0 { "#4D6BFE" } else { "#FF5252" };

    Some(DeepSeekResult {
        name: "DeepSeek".into(),
        r#type: "balance".into(),
        icon: "🟢".into(),
        balance: total,
        currency,
        unit: "元".into(),
        detail: format!("剩余 ¥{}", format_money(total)),
        color: color.into(),
    })
}

/// 格式化金额为千分位 + 两位小数。
fn format_money(v: f64) -> String {
    // 分整数/小数部分，整数加千分位逗号
    let s = format!("{:.2}", v);
    let (int_part, dec_part) = s.split_once('.').unwrap_or((&s, ""));
    let int_val: u64 = int_part.replace('-', "").parse().unwrap_or(0);
    let with_commas = insert_commas(int_val);
    if int_part.starts_with('-') {
        format!("-{with_commas}.{dec_part}")
    } else {
        format!("{with_commas}.{dec_part}")
    }
}

fn insert_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}
