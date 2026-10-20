//! 按 plan 实例并发拉取并组装展示快照。
//!
//! 数据流（替代旧的 balance.json 模板 + merge）：
//!   config::load_plans() → 逐实例 fetch_plan()（并发）→ build_snapshot() 保序组装
//!   → 快照存 AppState（全量）→ 出口（emit / get_services）按 plan.visible 过滤。
//!
//! 快照里每张卡片对应一个 plan 实例（ServiceInfo.id = plan.id），
//! 同品牌多实例天然支持，不存在旧版按名称前缀匹配的歧义。

use crate::config::PlanConfig;
use crate::data::{BalanceData, PlanFetch, ServiceInfo};
use crate::fetcher;

/// 分发一个 plan 实例到对应 fetcher。
///
/// 凭据统一经 `PlanConfig::credential()` 解析：普通实例取本地 key/cookie，
/// 内置实例按 builtin 序号从编译期密文解密（仅存在于本次调用的内存中）。
pub async fn fetch_plan(p: &PlanConfig) -> PlanFetch {
    let Some(cred) = p.credential() else {
        return PlanFetch::Skipped;
    };
    match p.vendor.as_str() {
        "glm" => fetcher::glm::fetch(&cred).await,
        "minimax" => fetcher::minimax::fetch(&cred).await,
        "mimo" => fetcher::mimo::fetch(&cred).await,
        "deepseek" => fetcher::deepseek::fetch(&cred).await,
        _ => PlanFetch::Skipped, // normalize_plans 已过滤，防御未知 vendor
    }
}

/// 按 plans 顺序组装快照：
///   - Ok：填入实例身份（id / vendor / label）后收录
///   - Failed：收录 loading 占位卡片（品牌默认名 / 图标 / 色），显示 N/A
///   - Skipped：未配置凭据，不出卡片
pub fn build_snapshot(plans: &[PlanConfig], results: Vec<PlanFetch>) -> BalanceData {
    let mut services = Vec::with_capacity(plans.len());
    for (p, r) in plans.iter().zip(results) {
        match r {
            PlanFetch::Ok(mut svc) => {
                svc.id = p.id.clone();
                svc.vendor = p.vendor.clone();
                svc.label = p.label.clone();
                services.push(svc);
            }
            PlanFetch::Failed => services.push(loading_service(p)),
            PlanFetch::Skipped => {}
        }
    }
    BalanceData {
        title: "AI API 余额监控".into(),
        services,
        timestamp: Some(BalanceData::now_timestamp()),
    }
}

/// 启动时的初始快照：已配置凭据的实例给 loading 卡片，等首轮拉取填充。
pub fn initial_snapshot(plans: &[PlanConfig]) -> BalanceData {
    let mut data = BalanceData {
        title: "AI API 余额监控".into(),
        services: plans
            .iter()
            .filter(|p| p.has_credential())
            .map(loading_service)
            .collect(),
        timestamp: None,
    };
    data.timestamp = Some(BalanceData::now_timestamp());
    data
}

/// Failed / 启动占位卡片：沿用品牌默认展示字段，loading=true（前端显示 N/A）。
fn loading_service(p: &PlanConfig) -> ServiceInfo {
    let (name, icon, color, ty) = vendor_defaults(&p.vendor);
    ServiceInfo {
        id: p.id.clone(),
        vendor: p.vendor.clone(),
        label: p.label.clone(),
        name,
        r#type: ty,
        icon,
        color,
        loading: true,
        ..Default::default()
    }
}

/// 品牌默认展示字段（fetch 失败时的 loading 卡片用）。
/// 成功数据的字段由各 fetcher 自带（含 GLM 的 level 后缀）。
fn vendor_defaults(vendor: &str) -> (String, String, String, String) {
    match vendor {
        "glm" => ("GLM 智谱AI".into(), "🔷".into(), "#2563EB".into(), "quota".into()),
        "minimax" => ("MiniMax".into(), "🟠".into(), "#EA580C".into(), "quota".into()),
        "mimo" => ("MiMo".into(), "🧡".into(), "#FF6900".into(), "quota".into()),
        "deepseek" => ("DeepSeek".into(), "🐳".into(), "#4D6BFE".into(), "balance".into()),
        _ => (String::new(), "●".into(), "#888888".into(), "quota".into()),
    }
}

/// 出口可见性过滤（展示层）：按 plan.visible 保留卡片。
/// 快照始终全量——取消隐藏可立即用缓存回显，随后刷新补数据。
/// 快照里找不到对应 plan 的条目防御性保留（normalize 之后的 plans 不应发生）。
pub fn filter_visible(data: &mut BalanceData, plans: &[PlanConfig]) {
    data.services.retain(|s| {
        plans
            .iter()
            .find(|p| p.id == s.id)
            .map(|p| p.visible)
            .unwrap_or(true)
    });
}

#[cfg(test)]
mod tests {
    use super::{build_snapshot, filter_visible, initial_snapshot, loading_service};
    use crate::config::PlanConfig;
    use crate::data::{PlanFetch, ServiceInfo};

    fn plan(id: &str, vendor: &str, visible: bool) -> PlanConfig {
        PlanConfig {
            id: id.into(),
            vendor: vendor.into(),
            label: String::new(),
            key: if vendor == "mimo" { String::new() } else { "k".into() },
            cookie: if vendor == "mimo" { "c".into() } else { String::new() },
            visible,
            builtin: None,
        }
    }

    fn ok_svc(id: &str, name: &str) -> ServiceInfo {
        ServiceInfo {
            id: id.into(),
            name: name.into(),
            r#type: "quota".into(),
            ..Default::default()
        }
    }

    #[test]
    fn snapshot_preserves_plan_order_and_identity() {
        let plans = vec![plan("glm", "glm", true), plan("mm", "minimax", true)];
        let results = vec![
            PlanFetch::Ok(ok_svc("glm", "GLM 智谱AI · GLM-4.5")),
            PlanFetch::Ok(ok_svc("mm", "MiniMax")),
        ];
        let data = build_snapshot(&plans, results);
        let names: Vec<&str> = data.services.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["GLM 智谱AI · GLM-4.5", "MiniMax"]);
        assert_eq!(data.services[0].vendor, "glm");
        assert_eq!(data.services[1].vendor, "minimax");
    }

    #[test]
    fn snapshot_failed_makes_loading_card_skipped_drops() {
        let plans = vec![plan("glm", "glm", true), plan("mimo", "mimo", true)];
        let results = vec![PlanFetch::Failed, PlanFetch::Skipped];
        let data = build_snapshot(&plans, results);
        assert_eq!(data.services.len(), 1, "Skipped 不出卡片");
        let svc = &data.services[0];
        assert_eq!(svc.id, "glm");
        assert!(svc.loading, "Failed 卡片标 loading");
        assert_eq!(svc.name, "GLM 智谱AI");
        assert_eq!(svc.color, "#2563EB");
    }

    #[test]
    fn snapshot_fills_label_from_plan() {
        let mut p = plan("glm-2", "glm", true);
        p.label = "备用".into();
        let data = build_snapshot(&[p], vec![PlanFetch::Ok(ok_svc("glm-2", "GLM 智谱AI"))]);
        assert_eq!(data.services[0].label, "备用");
    }

    #[test]
    fn initial_snapshot_only_for_credentialed_plans() {
        // 无 Cookie 的 mimo 实例不出卡片
        let mut no_cred = plan("mimo", "mimo", true);
        no_cred.cookie = String::new();
        let plans = vec![plan("glm", "glm", true), no_cred];
        let data = initial_snapshot(&plans);
        assert_eq!(data.services.len(), 1, "无 Cookie 的 mimo 不出卡片");
        assert!(data.services[0].loading);
        assert_eq!(data.services[0].id, "glm");
    }

    #[test]
    fn loading_card_keeps_label() {
        let mut p = plan("ds-1", "deepseek", true);
        p.label = "副号".into();
        let svc = loading_service(&p);
        assert_eq!(svc.label, "副号");
        assert_eq!(svc.r#type, "balance");
        assert!(svc.loading);
    }

    #[test]
    fn filter_keeps_only_visible_plans() {
        let plans = vec![plan("glm", "glm", true), plan("mm", "minimax", false)];
        let mut data = build_snapshot(
            &plans,
            vec![
                PlanFetch::Ok(ok_svc("glm", "GLM 智谱AI")),
                PlanFetch::Ok(ok_svc("mm", "MiniMax")),
            ],
        );
        assert_eq!(data.services.len(), 2);
        filter_visible(&mut data, &plans);
        let ids: Vec<&str> = data.services.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["glm"]);
    }

    #[test]
    fn filter_keeps_unknown_service_entries() {
        let plans = vec![plan("glm", "glm", true)];
        let mut data = build_snapshot(&plans, vec![PlanFetch::Ok(ok_svc("glm", "GLM 智谱AI"))]);
        data.services.push(ok_svc("ghost", "幽灵条目"));
        filter_visible(&mut data, &plans);
        assert_eq!(data.services.len(), 2, "无对应 plan 的条目防御性保留");
    }
}
