//! 把各家 fetcher 结果合并进 balance.json 读出的 BalanceData。
//!
//!
//! 核心规则（各家一致）：
//!   - 在 services 里找同名条目：
//!       找到 + 有数据 → 就地覆盖字段
//!       找到 + 无数据(None/Failed) → 标 loading=true（卡片显示 N/A 而非旧文件值）
//!   - 找不到 + 有数据 → 追加新条目
//!   - GLM 特殊：名称带套餐后缀「GLM 智谱AI · {LEVEL}」，匹配时认前缀
//!   - MiMo 特殊：三态（NotConfigured 删除条目，见 merge_mimo 注释）

use crate::data::{
    BalanceData, FetchOutcome, GlmResult, MimoFetch, MimoResult, MinimaxResult, QuotaItem,
    ServiceInfo,
};

/// 合并各家结果到 data（原地）。
pub fn merge_api_into(
    data: &mut BalanceData,
    outcome: &FetchOutcome,
) {
    merge_glm(data, outcome.glm.as_ref());
    merge_minimax(data, outcome.minimax.as_ref());
    merge_mimo(data, &outcome.mimo);
    // 清理历史残留条目：用户 balance.json 可能含已废弃服务（如旧版的 DeepSeek），
    // 这些服务不再有 fetcher 更新，会变成永不刷新的死卡片。这里只保留当前支持的
    // 三家（GLM / MiniMax / MiMo），与 merge 的名称匹配规则一致。
    data.services.retain(|s| {
        let n = s.name.trim();
        n.starts_with("GLM") || n.starts_with("MiniMax") || n.starts_with("MiMo")
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

/// MiMo 合并（三态，匹配认前缀 "MiMo"）。
///
/// 与另两家不同，MiMo 依赖用户运行期配置 Cookie，是可选服务：
///   - NotConfigured：删除现有条目——含 balance.json 模板里的占位卡片，
///     未启用的用户界面上不出现死卡片
///   - Failed：条目存在则标 loading（偶发失败保卡片显示 N/A），
///     不存在不追加（避免凭空出现来源不明的卡片）
///   - Ok：覆盖或追加（占位条也算有数据——「Cookie 已失效」提示需要可见）
fn merge_mimo(data: &mut BalanceData, mimo: &MimoFetch) {
    let pos = data.services.iter().position(|s| s.name.starts_with("MiMo"));

    match mimo {
        MimoFetch::NotConfigured => {
            if let Some(i) = pos {
                data.services.remove(i);
            }
        }
        MimoFetch::Failed => {
            if let Some(i) = pos {
                data.services[i].loading = true;
            }
        }
        MimoFetch::Ok(m) => merge_ok(data, pos, m),
    }
}

/// Ok 分支：覆盖现有条目或追加新条目。
fn merge_ok(data: &mut BalanceData, pos: Option<usize>, m: &MimoResult) {
    let items = m.items.clone();
    if items.is_empty() {
        // fetch 正常时不产出空 items，防御性兜底
        if let Some(i) = pos {
            data.services[i].loading = true;
        }
        return;
    }
    let icon = if m.icon.is_empty() {
        "🧡".to_string()
    } else {
        m.icon.clone()
    };
    let color = if m.color.is_empty() {
        "#FF6900".to_string()
    } else {
        m.color.clone()
    };
    match pos {
        Some(i) => {
            let svc = &mut data.services[i];
            svc.r#type = "quota".into();
            svc.items = items;
            svc.icon = icon;
            svc.color = color;
            svc.loading = false;
        }
        None => data.services.push(ServiceInfo {
            name: "MiMo".into(),
            r#type: "quota".into(),
            icon,
            color,
            items,
            ..Default::default()
        }),
    }
}
