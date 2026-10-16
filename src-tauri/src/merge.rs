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
//!   - DeepSeek 特殊：三态 + 余额型（见 merge_deepseek 注释）

use crate::data::{
    BalanceData, DeepseekFetch, DeepseekResult, FetchOutcome, GlmResult, MimoFetch, MimoResult,
    MinimaxResult, QuotaItem, ServiceInfo,
};

/// 合并各家结果到 data（原地）。
pub fn merge_api_into(
    data: &mut BalanceData,
    outcome: &FetchOutcome,
) {
    merge_glm(data, outcome.glm.as_ref());
    merge_minimax(data, outcome.minimax.as_ref());
    merge_mimo(data, &outcome.mimo);
    merge_deepseek(data, &outcome.deepseek);
    // 清理历史残留条目：用户 balance.json 可能含已下线监控的服务，
    // 这些服务不再有 fetcher 更新，会变成永不刷新的死卡片。这里只保留
    // 当前支持的四家（GLM / MiniMax / MiMo / DeepSeek），与 merge 的名称
    // 匹配规则一致。
    data.services.retain(|s| {
        let n = s.name.trim();
        n.starts_with("GLM")
            || n.starts_with("MiniMax")
            || n.starts_with("MiMo")
            || n.starts_with("DeepSeek")
    });
}

// ── 看板可见性过滤（用户在设置对话框勾选显示哪几家）──
//
// 快照（AppState.snapshot）始终保留全量合并结果，过滤只发生在展示层
// （get_services / services-updated emit），因此取消隐藏能立即用缓存数据回显，
// 无需等待下一轮拉取。

/// 卡片名称 → 服务商稳定 ID（与 merge_* 系列及上方 retain 的匹配规则一致）。
/// GLM 名称带套餐后缀（「GLM 智谱AI · XXX」）故认前缀。
pub fn plan_id_of(name: &str) -> Option<&'static str> {
    let n = name.trim();
    if n.starts_with("GLM") {
        Some("glm")
    } else if n.starts_with("MiniMax") {
        Some("minimax")
    } else if n.starts_with("MiMo") {
        Some("mimo")
    } else if n.starts_with("DeepSeek") {
        Some("deepseek")
    } else {
        None
    }
}

/// 按用户可见配置原地过滤卡片。visible 为稳定 ID 列表；
/// 空列表表示「全隐藏」——正常流程被 commands 层的「至少一个」校验挡住，
/// 这里不过滤（退化为全显示）以防手改配置后看板空白。
pub fn filter_visible_plans(data: &mut BalanceData, visible: &[String]) {
    if visible.is_empty() {
        return;
    }
    data.services.retain(|s| match plan_id_of(&s.name) {
        Some(id) => visible.iter().any(|v| v == id),
        // 未知条目不因可见性配置丢失（merge 已清理，防御性保留）
        None => true,
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

/// DeepSeek 合并（三态，匹配认前缀 "DeepSeek"）。
///
/// key 编译期注入、属可选服务（与 MiMo 的三态同思路，但凭据来源不同）：
///   - NotConfigured（二进制未内置 key）：删除现有条目——含 balance.json
///     模板里的占位卡片，未启用的用户界面上不出现死卡片
///   - Failed：条目存在则标 loading（偶发失败保卡片显示 N/A），
///     不存在不追加（避免凭空出现来源不明的卡片）
///   - Ok：覆盖或追加。余额型：写 balance/currency/detail，items 清空；
///     覆盖时把 r#type 纠正为 balance（兼容旧版手写的 quota 型条目）
fn merge_deepseek(data: &mut BalanceData, ds: &DeepseekFetch) {
    let pos = data
        .services
        .iter()
        .position(|s| s.name.starts_with("DeepSeek"));

    match ds {
        DeepseekFetch::NotConfigured => {
            if let Some(i) = pos {
                data.services.remove(i);
            }
        }
        DeepseekFetch::Failed => {
            if let Some(i) = pos {
                data.services[i].loading = true;
            }
        }
        DeepseekFetch::Ok(d) => merge_deepseek_ok(data, pos, d),
    }
}

/// Ok 分支：覆盖现有条目或追加新条目（余额型字段）。
fn merge_deepseek_ok(data: &mut BalanceData, pos: Option<usize>, d: &DeepseekResult) {
    let icon = if d.icon.is_empty() {
        "🐳".to_string()
    } else {
        d.icon.clone()
    };
    let color = if d.color.is_empty() {
        "#4D6BFE".to_string()
    } else {
        d.color.clone()
    };
    let currency = if d.currency.is_empty() {
        "¥".to_string()
    } else {
        d.currency.clone()
    };
    let unit = if d.unit.is_empty() {
        "元".to_string()
    } else {
        d.unit.clone()
    };
    match pos {
        Some(i) => {
            let svc = &mut data.services[i];
            svc.r#type = "balance".into();
            svc.items.clear();
            svc.balance = d.balance;
            svc.currency = currency;
            svc.unit = unit;
            svc.detail = d.detail.clone();
            svc.icon = icon;
            svc.color = color;
            svc.loading = false;
        }
        None => data.services.push(ServiceInfo {
            name: "DeepSeek".into(),
            r#type: "balance".into(),
            icon,
            color,
            balance: d.balance,
            currency,
            unit,
            detail: d.detail.clone(),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BalanceData {
        BalanceData {
            services: vec![
                ServiceInfo {
                    name: "GLM 智谱AI · GLM-4.5".into(),
                    ..Default::default()
                },
                ServiceInfo {
                    name: "MiniMax".into(),
                    ..Default::default()
                },
                ServiceInfo {
                    name: "MiMo".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn plan_id_maps_by_prefix() {
        assert_eq!(plan_id_of("GLM 智谱AI"), Some("glm"));
        assert_eq!(plan_id_of("GLM 智谱AI · GLM-4.5"), Some("glm"));
        assert_eq!(plan_id_of("MiniMax"), Some("minimax"));
        assert_eq!(plan_id_of("MiniMax 稀宇科技"), Some("minimax"));
        assert_eq!(plan_id_of("MiMo"), Some("mimo"));
        assert_eq!(plan_id_of("DeepSeek"), Some("deepseek"));
        // MiniMax 不误匹配 MiMo 前缀
        assert_ne!(plan_id_of("MiniMax"), Some("mimo"));
        assert_eq!(plan_id_of("OpenAI"), None);
    }

    #[test]
    fn deepseek_not_configured_removes_entry() {
        let mut data = sample();
        data.services.push(ServiceInfo {
            name: "DeepSeek".into(),
            ..Default::default()
        });
        merge_deepseek(&mut data, &DeepseekFetch::NotConfigured);
        assert!(!data.services.iter().any(|s| s.name == "DeepSeek"));
    }

    #[test]
    fn deepseek_failed_marks_loading_but_never_appends() {
        // 条目存在 → 标 loading（卡片显示 N/A）
        let mut data = sample();
        data.services.push(ServiceInfo {
            name: "DeepSeek".into(),
            ..Default::default()
        });
        merge_deepseek(&mut data, &DeepseekFetch::Failed);
        assert!(data.services.last().unwrap().loading);

        // 条目不存在 → 不追加（不凭空出现死卡片）
        let mut data = sample();
        merge_deepseek(&mut data, &DeepseekFetch::Failed);
        assert!(!data.services.iter().any(|s| s.name == "DeepSeek"));
    }

    #[test]
    fn deepseek_ok_overwrites_as_balance_type() {
        let mut data = sample();
        // 模拟旧版手写的 quota 型 DeepSeek 条目：合并时纠正为 balance
        data.services.push(ServiceInfo {
            name: "DeepSeek".into(),
            r#type: "quota".into(),
            items: vec![QuotaItem::default()],
            ..Default::default()
        });
        let ok = DeepseekFetch::Ok(DeepseekResult {
            name: "DeepSeek".into(),
            r#type: "balance".into(),
            icon: "🐳".into(),
            color: "#4D6BFE".into(),
            balance: 110.0,
            currency: "¥".into(),
            unit: "元".into(),
            detail: "充值 ¥100.00 · 赠送 ¥10.00".into(),
        });
        merge_deepseek(&mut data, &ok);

        let svc = data.services.last().unwrap();
        assert_eq!(svc.r#type, "balance");
        assert!(!svc.loading);
        assert!(svc.items.is_empty());
        assert_eq!(svc.balance, 110.0);
        assert_eq!(svc.currency, "¥");
        assert_eq!(svc.detail, "充值 ¥100.00 · 赠送 ¥10.00");
    }

    #[test]
    fn deepseek_ok_appends_when_missing() {
        let mut data = sample();
        let ok = DeepseekFetch::Ok(DeepseekResult {
            name: "DeepSeek".into(),
            r#type: "balance".into(),
            icon: String::new(),
            color: String::new(),
            balance: 5.5,
            currency: String::new(),
            unit: String::new(),
            detail: "暂无余额".into(),
        });
        merge_deepseek(&mut data, &ok);
        let svc = data.services.last().unwrap();
        assert_eq!(svc.name, "DeepSeek");
        // 空 icon/color/currency/unit 用默认兜底
        assert_eq!(svc.icon, "🐳");
        assert_eq!(svc.color, "#4D6BFE");
        assert_eq!(svc.currency, "¥");
        assert_eq!(svc.unit, "元");
        assert_eq!(svc.balance, 5.5);
    }

    #[test]
    fn retain_keeps_all_four_supported_services() {
        let mut data = sample();
        data.services.push(ServiceInfo {
            name: "DeepSeek".into(),
            ..Default::default()
        });
        data.services.push(ServiceInfo {
            name: "已下家的旧服务".into(),
            ..Default::default()
        });
        merge_api_into(
            &mut data,
            &FetchOutcome {
                glm: None,
                minimax: None,
                mimo: crate::data::MimoFetch::NotConfigured,
                deepseek: DeepseekFetch::NotConfigured,
            },
        );
        // NotConfigured 移除 MiMo/DeepSeek；未知名称被 retain 清掉
        assert!(data.services.iter().all(|s| {
            let n = s.name.as_str();
            n.starts_with("GLM") || n.starts_with("MiniMax")
        }));
    }

    #[test]
    fn filter_keeps_only_visible() {
        let visible = vec!["glm".to_string(), "mimo".to_string()];
        let mut data = sample();
        filter_visible_plans(&mut data, &visible);
        let names: Vec<&str> = data.services.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["GLM 智谱AI · GLM-4.5", "MiMo"]);
    }

    #[test]
    fn filter_empty_visible_keeps_all() {
        let mut data = sample();
        filter_visible_plans(&mut data, &[]);
        assert_eq!(data.services.len(), 3);
    }

    #[test]
    fn filter_single_visible() {
        let visible = vec!["minimax".to_string()];
        let mut data = sample();
        filter_visible_plans(&mut data, &visible);
        assert_eq!(data.services.len(), 1);
        assert_eq!(data.services[0].name, "MiniMax");
    }
}
