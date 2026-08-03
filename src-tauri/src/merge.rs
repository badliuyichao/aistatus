//! 把两家 fetcher 结果合并进 balance.json 读出的 BalanceData。
//!
//!
//! 核心规则（两家一致）：
//!   - 在 services 里找同名条目：
//!       找到 + 有数据 → 就地覆盖字段
//!       找到 + 无数据(None) → 标 loading=true（卡片显示 N/A 而非旧文件值）
//!   - 找不到 + 有数据 → 追加新条目
//!   - GLM 特殊：名称带套餐后缀「GLM 智谱AI · {LEVEL}」，匹配时认前缀

use crate::data::{
    BalanceData, FetchOutcome, GlmResult, MinimaxResult, QuotaItem, ServiceInfo,
};

/// 合并两家结果到 data（原地）。
pub fn merge_api_into(
    data: &mut BalanceData,
    outcome: &FetchOutcome,
) {
    merge_glm(data, outcome.glm.as_ref());
    merge_minimax(data, outcome.minimax.as_ref());
    // 清理历史残留条目：用户 balance.json 可能含已废弃服务（如旧版的 DeepSeek），
    // 这些服务不再有 fetcher 更新，会变成永不刷新的死卡片。这里只保留当前支持的
    // 两家（GLM / MiniMax），与 merge 的名称匹配规则一致。
    data.services.retain(|s| {
        let n = s.name.trim();
        n.starts_with("GLM") || n.starts_with("MiniMax")
    });
}

/// GLM 合并。
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

/// MiniMax 合并。
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
