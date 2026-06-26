//! 把三家 fetcher 结果合并进 balance.json 读出的 BalanceData。
//!
//! 对应 legacy `data_manager.py` 的 `merge_api_into` + `_merge_deepseek` /
//! `_merge_glm` / `_merge_minimax`。
//!
//! 核心规则（三家一致）：
//!   - 在 services 里找同名条目：
//!       找到 + 有数据 → 就地覆盖字段
//!       找到 + 无数据(None) → 标 loading=true（卡片显示 N/A 而非旧文件值）
//!   - 找不到 + 有数据 → 追加新条目
//!   - GLM 特殊：名称带套餐后缀「GLM 智谱AI · {LEVEL}」，匹配时认前缀

use crate::data::{
    BalanceData, DeepSeekResult, FetchOutcome, GlmResult, MinimaxResult, QuotaItem, ServiceInfo,
};

/// 合并三家结果到 data（原地）。对应 Python `merge_api_into`。
pub fn merge_api_into(
    data: &mut BalanceData,
    outcome: &FetchOutcome,
) {
    merge_deepseek(data, outcome.deepseek.as_ref());
    merge_glm(data, outcome.glm.as_ref());
    merge_minimax(data, outcome.minimax.as_ref());
}

/// DeepSeek 合并。对应 Python `_merge_deepseek`。
fn merge_deepseek(data: &mut BalanceData, ds: Option<&DeepSeekResult>) {
    // 找现有 DeepSeek 条目
    if let Some(svc) = data.services.iter_mut().find(|s| s.name == "DeepSeek") {
        match ds {
            None => svc.loading = true,
            Some(d) => {
                svc.r#type = d.r#type.clone();
                svc.balance = d.balance;
                svc.currency = d.currency.clone();
                svc.unit = d.unit.clone();
                svc.detail = d.detail.clone();
                svc.color = d.color.clone();
                svc.loading = false;
            }
        }
        return;
    }

    // 无现有条目 → 有数据才追加
    if let Some(d) = ds {
        data.services.push(ServiceInfo {
            name: "DeepSeek".into(),
            r#type: d.r#type.clone(),
            icon: d.icon.clone(),
            color: d.color.clone(),
            balance: d.balance,
            currency: d.currency.clone(),
            unit: d.unit.clone(),
            detail: d.detail.clone(),
            ..Default::default()
        });
    }
}

/// GLM 合并。对应 Python `_merge_glm`。
/// 匹配认前缀（"GLM 智谱AI" 或 "GLM 智谱AI · "）以兼容带后缀的名称。
fn merge_glm(data: &mut BalanceData, glm: Option<&GlmResult>) {
    // 找现有 GLM 条目（认前缀）
    let pos = data
        .services
        .iter()
        .position(|s| s.name == "GLM 智谱AI" || s.name.starts_with("GLM 智谱AI · "));

    if let Some(i) = pos {
        let svc = &mut data.services[i];
        match glm {
            None => svc.loading = true,
            Some(g) => {
                let items: Vec<QuotaItem> = g.items.clone();
                if items.is_empty() {
                    svc.loading = true;
                    return;
                }
                // 标题后缀：level 非空 → 「GLM 智谱AI · {LEVEL 大写}」
                let display_name = match &g.level {
                    Some(lv) if !lv.is_empty() => format!("GLM 智谱AI · {}", lv.to_uppercase()),
                    _ => "GLM 智谱AI".into(),
                };
                svc.r#type = "quota".into();
                svc.items = items;
                svc.color = if g.color.is_empty() {
                    "#2563EB".into()
                } else {
                    g.color.clone()
                };
                if !g.icon.is_empty() {
                    svc.icon = g.icon.clone();
                }
                svc.name = display_name;
                svc.loading = false;
            }
        }
        return;
    }

    // 无现有条目 → 有数据才追加
    if let Some(g) = glm {
        let items = g.items.clone();
        if items.is_empty() {
            return;
        }
        let display_name = match &g.level {
            Some(lv) if !lv.is_empty() => format!("GLM 智谱AI · {}", lv.to_uppercase()),
            _ => "GLM 智谱AI".into(),
        };
        data.services.push(ServiceInfo {
            name: display_name,
            r#type: "quota".into(),
            icon: if g.icon.is_empty() {
                "🔷".into()
            } else {
                g.icon.clone()
            },
            color: if g.color.is_empty() {
                "#2563EB".into()
            } else {
                g.color.clone()
            },
            items,
            ..Default::default()
        });
    }
}

/// MiniMax 合并。对应 Python `_merge_minimax`。
/// 匹配 "MiniMax" 或旧名 "MiniMax 稀宇科技"。
fn merge_minimax(data: &mut BalanceData, mm: Option<&MinimaxResult>) {
    let pos = data
        .services
        .iter()
        .position(|s| s.name == "MiniMax" || s.name == "MiniMax 稀宇科技");

    if let Some(i) = pos {
        let svc = &mut data.services[i];
        match mm {
            None => svc.loading = true,
            Some(m) => {
                let items = m.items.clone();
                if items.is_empty() {
                    svc.loading = true;
                    return;
                }
                svc.r#type = "quota".into();
                svc.items = items;
                svc.color = if m.color.is_empty() {
                    "#EA580C".into()
                } else {
                    m.color.clone()
                };
                if !m.icon.is_empty() {
                    svc.icon = m.icon.clone();
                }
                svc.loading = false;
            }
        }
        return;
    }

    if let Some(m) = mm {
        let items = m.items.clone();
        if items.is_empty() {
            return;
        }
        data.services.push(ServiceInfo {
            name: "MiniMax".into(),
            r#type: "quota".into(),
            icon: if m.icon.is_empty() {
                "🟠".into()
            } else {
                m.icon.clone()
            },
            color: if m.color.is_empty() {
                "#EA580C".into()
            } else {
                m.color.clone()
            },
            items,
            ..Default::default()
        });
    }
}
