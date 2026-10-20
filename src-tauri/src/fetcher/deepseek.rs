//! DeepSeek 开放平台余额查询。
//!
//! URL:  https://api.deepseek.com/user/balance   （官方公开接口）
//! Auth: Bearer <api_key>（用户在设置对话框运行期配置，见 config::PlanConfig）
//!
//! 返回三态 PlanFetch（见 data.rs）：
//!   - Skipped：未配置 key
//!   - Failed：网络 / 其他 HTTP 失败（卡片 N/A）
//!   - Ok：成功数据，或明确错误占位（401 key 无效 / 402、503 余额不足或繁忙，
//!     给可见提示而非 N/A——对齐 MiMo「Cookie 已失效」占位的处理思路）
//!
//! 解析注意：
//!   - 金额字段是字符串（"110.00"），且 granted / topped_up 可能缺省 → 全按
//!     Option 解析、解析失败记 0
//!   - balance_infos 理论上可含多币种条目 → 优先取 CNY，否则取第一条
//!   - is_available=false 表示账户暂不可用（余额不足 / 欠费），拼进 detail 提示

use reqwest::header::{ACCEPT, AUTHORIZATION};

use crate::data::{PlanFetch, ServiceInfo};

const DEEPSEEK_BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

/// 官方响应体（只取需要的字段）。
#[derive(serde::Deserialize, Debug, Default)]
struct DeepseekResp {
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(serde::Deserialize, Debug, Default)]
struct BalanceInfo {
    #[serde(default)]
    currency: String,
    #[serde(default)]
    total_balance: Option<String>,
    #[serde(default)]
    granted_balance: Option<String>,
    #[serde(default)]
    topped_up_balance: Option<String>,
}

/// 拉取 DeepSeek 余额。
pub async fn fetch(api_key: &str) -> PlanFetch {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return PlanFetch::Skipped;
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
            eprintln!("[DeepseekFetcher] network error: {e}");
            return PlanFetch::Failed;
        }
    };

    let status = resp.status();
    if !status.is_success() {
        // 明确错误给占位提示（对齐 MiMo Cookie 失效的处理）：比 N/A 更可诊断
        match status.as_u16() {
            // 401：认证失败（key 无效 / 被删）
            401 => {
                eprintln!("[DeepseekFetcher] HTTP 401 (invalid api key)");
                return PlanFetch::Ok(placeholder_service(
                    "API Key 无效 · 请在设置中检查".into(),
                ));
            }
            // 402/503：余额不足或服务繁忙（官方文档仅列出这两个非认证错误码）
            402 | 503 => {
                eprintln!("[DeepseekFetcher] HTTP {status} (insufficient balance or busy)");
                return PlanFetch::Ok(placeholder_service("余额不足或服务繁忙".into()));
            }
            _ => {
                eprintln!("[DeepseekFetcher] HTTP {status}");
                return PlanFetch::Failed;
            }
        }
    }

    let body: DeepseekResp = match resp.json().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[DeepseekFetcher] invalid JSON response: {e}");
            return PlanFetch::Failed;
        }
    };

    // 多币种时优先 CNY（国内主账户），否则退回第一条
    let info = body
        .balance_infos
        .iter()
        .find(|b| b.currency == "CNY")
        .or_else(|| body.balance_infos.first());
    let Some(info) = info else {
        eprintln!("[DeepseekFetcher] no balance_infos in response");
        return PlanFetch::Ok(placeholder_service("暂无余额数据".into()));
    };

    let total = parse_amount(&info.total_balance);
    let granted = parse_amount(&info.granted_balance);
    let topped = parse_amount(&info.topped_up_balance);
    let sym = currency_symbol(&info.currency);

    PlanFetch::Ok(ServiceInfo {
        name: "DeepSeek".into(),
        r#type: "balance".into(),
        vendor: "deepseek".into(),
        icon: "🐳".into(),
        color: "#4D6BFE".into(),
        balance: total,
        currency: sym.clone(),
        unit: "元".into(),
        detail: compose_detail(&sym, topped, granted, body.is_available),
        placeholder: false,
        ..Default::default()
    })
}

/// 解析字符串金额（"110.00"，容错千分位逗号）；缺失 / 非法记 0。
fn parse_amount(v: &Option<String>) -> f64 {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.replace(',', "").parse::<f64>().ok())
        .unwrap_or(0.0)
}

/// 币种代码 → 展示符号。未知币种原样返回（BalanceView 直接拼接在金额前）。
fn currency_symbol(code: &str) -> String {
    match code {
        "CNY" | "RMB" => "¥".into(),
        "USD" => "$".into(),
        "" => "¥".into(),
        other => format!("{other} "),
    }
}

/// 组装明细文案：充值 / 赠送拆分（零值段不显示），账户不可用时附提示。
fn compose_detail(sym: &str, topped: f64, granted: f64, available: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    if topped > 0.0 {
        parts.push(format!("充值 {sym}{topped:.2}"));
    }
    if granted > 0.0 {
        parts.push(format!("赠送 {sym}{granted:.2}"));
    }
    if parts.is_empty() {
        parts.push("暂无余额".into());
    }
    if !available {
        parts.push("余额不足，请及时充值".into());
    }
    parts.join(" · ")
}

/// 占位结果（key 无效 / 余额不足 / 无数据时）。金额给 0，
/// 靠 detail 说明原因——与 MiMo 占位条同思路，保证提示可见。
fn placeholder_service(msg: String) -> ServiceInfo {
    ServiceInfo {
        name: "DeepSeek".into(),
        r#type: "balance".into(),
        vendor: "deepseek".into(),
        icon: "🐳".into(),
        color: "#4D6BFE".into(),
        balance: 0.0,
        currency: "¥".into(),
        unit: "元".into(),
        detail: msg,
        placeholder: true,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_official_response() {
        let resp: DeepseekResp = serde_json::from_str(
            r#"{
                "is_available": true,
                "balance_infos": [
                    { "currency": "CNY", "total_balance": "110.00",
                      "granted_balance": "10.00", "topped_up_balance": "100.00" }
                ]
            }"#,
        )
        .unwrap();
        assert!(resp.is_available);
        assert_eq!(resp.balance_infos.len(), 1);
        assert_eq!(parse_amount(&resp.balance_infos[0].total_balance), 110.0);
        assert_eq!(parse_amount(&resp.balance_infos[0].granted_balance), 10.0);
        assert_eq!(
            parse_amount(&resp.balance_infos[0].topped_up_balance),
            100.0
        );
    }

    #[test]
    fn parses_missing_amount_fields_as_zero() {
        let resp: DeepseekResp = serde_json::from_str(
            r#"{ "is_available": false, "balance_infos": [ { "currency": "CNY", "total_balance": "0.00" } ] }"#,
        )
        .unwrap();
        assert!(!resp.is_available);
        assert_eq!(parse_amount(&resp.balance_infos[0].total_balance), 0.0);
        assert_eq!(parse_amount(&resp.balance_infos[0].granted_balance), 0.0);
    }

    #[test]
    fn amount_tolerates_comma_thousands() {
        assert_eq!(parse_amount(&Some("1,234.56".into())), 1234.56);
        assert_eq!(parse_amount(&Some(String::new())), 0.0);
        assert_eq!(parse_amount(&None), 0.0);
        assert_eq!(parse_amount(&Some("n/a".into())), 0.0);
    }

    #[test]
    fn currency_maps_to_symbol() {
        assert_eq!(currency_symbol("CNY"), "¥");
        assert_eq!(currency_symbol("RMB"), "¥");
        assert_eq!(currency_symbol("USD"), "$");
        assert_eq!(currency_symbol(""), "¥");
        assert_eq!(currency_symbol("EUR"), "EUR ");
    }

    #[test]
    fn detail_composes_topup_grant_and_availability() {
        // 正常：充值 + 赠送
        assert_eq!(
            compose_detail("¥", 100.0, 10.0, true),
            "充值 ¥100.00 · 赠送 ¥10.00"
        );
        // 零值段不显示；全零给「暂无余额」
        assert_eq!(compose_detail("¥", 0.0, 0.0, true), "暂无余额");
        assert_eq!(
            compose_detail("¥", 0.0, 5.0, false),
            "赠送 ¥5.00 · 余额不足，请及时充值"
        );
    }
}
