//! config.json 读写 —— 存放主题偏好、悬浮条位置与 Coding Plan 实例列表。
//!
//! API Key 自多实例改造起由用户**运行期配置**：明文存于本文件 `plans` 数组
//! （与 MiMo Cookie 同一威胁模型——config.json 只落用户本机数据目录、不进
//! git，无需混淆）。编译期加密的内置 key（secrets 模块）退化为**播种源**：
//! config.json 尚无 `plans` 字段时（升级 / 全新环境首次启动），用内置 key +
//! 旧 visiblePlans + 旧 mimo.cookie 生成初始实例，之后 config.json 是唯一真相源。
//!
//! 路径：跨平台用户配置目录下的 `aistatus/config.json`
//!   - macOS: ~/Library/Application Support/aistatus/config.json
//!   - Windows: %APPDATA%\aistatus\config.json
//!   - Linux: ~/.config/aistatus/config.json（预留）
//!
//! `dirs::config_dir()` 负责按平台返回符合系统约定的配置目录。

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 用户数据根目录：`<config_dir>/aistatus/`。
pub fn data_dir() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("aistatus")
}

/// config.json 完整路径。
pub fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

#[allow(dead_code)]
pub fn ensure_data_dir() -> std::io::Result<()> {
    fs::create_dir_all(data_dir())
}

// ── 主题偏好（config.json 的 theme 字段）──

/// 读主题偏好；不存在 / 损坏 / 非法值返回 "system"。
/// 对应前端 Theme = "system" | "dark" | "light"。
pub fn load_theme() -> String {
    let value = read_config_value();
    let t = value
        .get("theme")
        .and_then(|v| v.as_str())
        .unwrap_or("system");
    if matches!(t, "system" | "dark" | "light") {
        t.to_string()
    } else {
        "system".to_string()
    }
}

/// 写主题偏好：读-改-写（保留其他字段）+ 原子 tmp+rename。
pub fn save_theme(theme: &str) -> std::io::Result<()> {
    write_config_value("theme", serde_json::json!(theme))
}

/// 通用：读-改-写 config.json 的一个顶层字段（保留其他字段）+ 原子 tmp+rename。
fn write_config_value(key: &str, value: serde_json::Value) -> std::io::Result<()> {
    let mut root = read_config_value();
    if let Some(obj) = root.as_object_mut() {
        obj.insert(key.into(), value);
    }
    write_config_root(root)
}

/// 写整个 config.json 根对象 + 原子 tmp+rename（迁移播种整树替换用）。
fn write_config_root(root: serde_json::Value) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&root)?)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// 读 config.json 为 serde_json::Value；不存在 / 损坏 / 非 object 返回空 object。
/// 供各字段的读-改-写用。
fn read_config_value() -> serde_json::Value {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|b| serde_json::from_str(&b).ok())
        .filter(|v: &serde_json::Value| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

// ── Coding Plan 实例（config.json 的 plans 字段）──
//
// 同品牌可配多个实例（如 2 个 GLM coding plan、2 个 DeepSeek key）。
// glm / minimax / deepseek 用 key 字段；mimo 接口只认浏览器 Cookie，用
// cookie 字段。每实例独立 visible（展示层出口过滤，主窗口与悬浮条同步显隐）。
// 「至少保留一个实例 / 至少一个可见」的校验在 commands::save_plans 与前端 UI。

/// 支持的服务商稳定 vendor ID。实例的 vendor 字段以此为白名单。
pub const VENDORS: [&str; 4] = ["glm", "minimax", "mimo", "deepseek"];

/// 一个 Coding Plan 实例。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanConfig {
    /// 实例稳定 ID（`{vendor}-{6位hex}`），跨重启不变，卡片 / 悬浮条 / 显隐以它标识。
    #[serde(default)]
    pub id: String,
    /// 服务商：glm / minimax / mimo / deepseek（VENDORS 白名单）。
    #[serde(default)]
    pub vendor: String,
    /// 用户命名的实例标签（可空），用于区分同品牌实例（如「主号」「备用」）。
    #[serde(default)]
    pub label: String,
    /// API Key（glm / minimax / deepseek 用户自行添加的实例）。mimo 不使用。
    ///
    /// **内置实例（分发场景）此字段恒为空**——key 只以密文编在二进制里，
    /// 永不落盘，凭据经 `builtin` 序号引用（见下），拉取时才在内存解密。
    #[serde(default)]
    pub key: String,
    /// MiMo 登录 Cookie（仅 vendor == "mimo" 使用）。
    #[serde(default)]
    pub cookie: String,
    /// 是否显示在主窗口与悬浮条。字段缺失按 true（老配置兼容）。
    #[serde(default = "default_true")]
    pub visible: bool,
    /// 内置实例引用：使用编译进二进制的该品牌第 N 个 key（secrets 模块按序
    /// 解密）。`key` 字段必须为空。None = 普通实例（key 明文本地，用户自配）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builtin: Option<usize>,
}

fn default_true() -> bool {
    true
}

impl PlanConfig {
    /// 实例的实际凭据：内置实例从编译期密文解密（仅在内存短暂存在），
    /// 普通实例用本地明文 key / cookie。None = 未配置凭据（不出卡片）。
    pub fn credential(&self) -> Option<String> {
        if self.vendor == "mimo" {
            return non_empty(self.cookie.clone());
        }
        if let Some(idx) = self.builtin {
            let keys = match self.vendor.as_str() {
                "glm" => crate::secrets::glm_keys(),
                "minimax" => crate::secrets::minimax_keys(),
                "deepseek" => crate::secrets::deepseek_keys(),
                _ => Vec::new(),
            };
            // 越界（如无内置 key 的构建 + 手改配置）→ None，不出卡片
            return keys.into_iter().nth(idx).and_then(non_empty);
        }
        non_empty(self.key.clone())
    }

    /// 实例是否配置了凭据（决定出不出卡片）。
    pub fn has_credential(&self) -> bool {
        self.credential().is_some()
    }
}

fn non_empty(s: String) -> Option<String> {
    let t = s.trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// 生成实例 ID：`{vendor}-{6位hex}`（纳秒时间戳派生）。手工添加频率极低，
/// 24bit 空间 + normalize 的去重重试足够避撞；删除后不复用。
pub fn new_plan_id(vendor: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{}-{:06x}", vendor, nanos & 0xFF_FFFF)
}

/// 规范化 plan 列表：trim 字段、vendor 小写白名单过滤、空 / 重复 id 补生成。
/// load 与 save 共用，保证内存与磁盘上的列表始终规范（手改文件也兜底）。
pub fn normalize_plans(raw: Vec<PlanConfig>) -> Vec<PlanConfig> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(raw.len());
    for mut p in raw {
        p.vendor = p.vendor.trim().to_lowercase();
        if !VENDORS.contains(&p.vendor.as_str()) {
            continue; // 未知服务商静默丢弃（防手调接口 / 手改文件传垃圾值）
        }
        p.id = p.id.trim().to_string();
        if p.id.is_empty() || !seen.insert(p.id.clone()) {
            loop {
                p.id = new_plan_id(&p.vendor);
                if seen.insert(p.id.clone()) {
                    break;
                }
            }
        }
        p.label = p.label.trim().to_string();
        p.key = p.key.trim().to_string();
        p.cookie = p.cookie.trim().to_string();
        // 内置实例的凭据走编译期密文，key 字段必须保持为空
        // （防止 UI「换 Key」残留 / 手改文件把明文写进内置实例）。
        if p.builtin.is_some() {
            p.key.clear();
        }
        out.push(p);
    }
    out
}

/// 读 plan 列表（已规范化）。config.json 无 `plans` 字段时执行一次性播种迁移；
/// 已有 plans 时做一次「内置 key 明文自清理」（误存的内置 key 转为 builtin
/// 引用并从盘上清除，见 strip_builtin_key_copies），有变更立即落盘。
/// 每次刷新现读文件，无需缓存失效（与旧 MiMo Cookie 同思路）。
pub fn load_plans() -> Vec<PlanConfig> {
    let root = read_config_value();
    match root.get("plans") {
        Some(v) => {
            let raw: Vec<PlanConfig> = serde_json::from_value(v.clone()).unwrap_or_default();
            let (plans, changed) = strip_builtin_key_copies(normalize_plans(raw));
            if changed {
                let _ = save_plans(&plans);
            }
            plans
        }
        None => migrate_seed_plans(root),
    }
}

/// 升级迁移：用编译期内置 key + 旧 visiblePlans + 旧 mimo.cookie 播种初始实例，
/// 立即写回 config.json（同时移除已废弃的 visiblePlans 字段）。之后不再进入本函数。
fn migrate_seed_plans(root: serde_json::Value) -> Vec<PlanConfig> {
    let visible_ids = legacy_visible_ids(&root);
    let cookie = root
        .get("mimo")
        .and_then(|m| m.get("cookie"))
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let plans = seed_plans(
        &crate::secrets::glm_keys(),
        &crate::secrets::minimax_keys(),
        &crate::secrets::deepseek_keys(),
        cookie,
        &visible_ids,
    );
    let mut obj = root.as_object().cloned().unwrap_or_default();
    obj.remove("visiblePlans");
    if let Ok(v) = serde_json::to_value(&plans) {
        obj.insert("plans".into(), v);
    }
    let _ = write_config_root(serde_json::Value::Object(obj));
    plans
}

/// 读旧 visiblePlans 字段（播种迁移用）：trim + 小写 + 白名单 + 去重；
/// 缺失 / 空 / 全非法 → 全部 vendor（默认全显，对齐旧 load_visible_plans 兜底）。
fn legacy_visible_ids(root: &serde_json::Value) -> Vec<String> {
    let raw: Vec<String> = root
        .get("visiblePlans")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_lowercase())
                .filter(|s| VENDORS.contains(&s.as_str()))
                .collect()
        })
        .unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let ids: Vec<String> = raw.into_iter().filter(|s| seen.insert(s.clone())).collect();
    if ids.is_empty() {
        VENDORS.iter().map(|s| s.to_string()).collect()
    } else {
        ids
    }
}

/// 纯函数播种（key 列表作参数便于测试）：每个内置 key 建一个实例，
/// 以 `{vendor}-b{序号}` 为 id、`builtin: Some(序号)` 引用编译期密文，
/// **key 字段保持为空**（内置凭据永不落盘）。Cookie 非空才建 MiMo 条目
/// （明文本地，MiMo 接口只认 Cookie、无内置概念）。
/// visible_ids 为空视为全显。
fn seed_plans(
    glm_keys: &[String],
    minimax_keys: &[String],
    deepseek_keys: &[String],
    mimo_cookie: &str,
    visible_ids: &[String],
) -> Vec<PlanConfig> {
    let visible = |vendor: &str| visible_ids.is_empty() || visible_ids.iter().any(|v| v == vendor);
    let mut plans = Vec::new();
    let mut push_builtin = |vendor: &str, idx: usize| {
        plans.push(PlanConfig {
            id: format!("{}-b{}", vendor, idx),
            vendor: vendor.to_string(),
            label: String::new(),
            key: String::new(),
            cookie: String::new(),
            visible: visible(vendor),
            builtin: Some(idx),
        });
    };
    for i in 0..glm_keys.len() {
        push_builtin("glm", i);
    }
    for i in 0..minimax_keys.len() {
        push_builtin("minimax", i);
    }
    for i in 0..deepseek_keys.len() {
        push_builtin("deepseek", i);
    }
    if !mimo_cookie.trim().is_empty() {
        plans.push(PlanConfig {
            id: "mimo".into(),
            vendor: "mimo".into(),
            label: String::new(),
            key: String::new(),
            cookie: mimo_cookie.trim().to_string(),
            visible: visible("mimo"),
            builtin: None,
        });
    }
    plans
}

/// 把 config.json 里与内置 key 相同的明文转换为 builtin 引用并清除明文。
/// 场景：多实例初版（key 明文落盘）升级到内置引用模型后的自清理；
/// 重复把内置 key 粘成普通实例的也一并收敛。每轮 load 都会跑，
/// 有变更才落盘。纯内核 + 薄包装（keys 作参数便于测试）。
fn strip_builtin_key_copies(plans: Vec<PlanConfig>) -> (Vec<PlanConfig>, bool) {
    strip_with(
        plans,
        &crate::secrets::glm_keys(),
        &crate::secrets::minimax_keys(),
        &crate::secrets::deepseek_keys(),
    )
}

fn strip_with(
    mut plans: Vec<PlanConfig>,
    glm: &[String],
    minimax: &[String],
    deepseek: &[String],
) -> (Vec<PlanConfig>, bool) {
    let mut changed = false;
    for p in &mut plans {
        if p.builtin.is_some() || p.key.is_empty() || p.vendor == "mimo" {
            continue;
        }
        let keys = match p.vendor.as_str() {
            "glm" => glm,
            "minimax" => minimax,
            "deepseek" => deepseek,
            _ => continue,
        };
        if let Some(idx) = keys.iter().position(|k| k == &p.key) {
            p.builtin = Some(idx);
            p.key.clear();
            changed = true;
        }
    }
    (plans, changed)
}

/// 写 plan 列表：读-改-写（保留其他字段）+ 原子 tmp+rename。
/// 规范化与非空校验由调用方负责（commands::save_plans）。
pub fn save_plans(plans: &[PlanConfig]) -> std::io::Result<()> {
    write_config_value("plans", serde_json::json!(plans))
}

// ── 小米 MiMo 登录 Cookie（plans 中 vendor=="mimo" 条目的 cookie 字段）──

/// 读 MiMo Cookie；无 mimo 实例 / 未配置返回空串（fetcher 跳过不出卡片）。
///
/// 明文存储：config.json 只落在用户本机数据目录、不进 git，与编译期加密
/// （防 key 进分发包被 strings 提取）的威胁模型不同，无需混淆。
pub fn load_mimo_cookie() -> String {
    load_plans()
        .into_iter()
        .find(|p| p.vendor == "mimo")
        .map(|p| p.cookie)
        .unwrap_or_default()
}

/// 写 MiMo Cookie：更新（或创建）mimo 实例的 cookie 字段。
/// 置空串 = 清除配置（实例保留，卡片消失，可在设置里重新粘贴）。
pub fn save_mimo_cookie(cookie: &str) -> std::io::Result<()> {
    let mut plans = load_plans();
    match plans.iter_mut().find(|p| p.vendor == "mimo") {
        Some(p) => p.cookie = cookie.trim().to_string(),
        None => plans.push(PlanConfig {
            id: new_plan_id("mimo"),
            vendor: "mimo".into(),
            label: String::new(),
            key: String::new(),
            cookie: cookie.trim().to_string(),
            visible: true,
            builtin: None,
        }),
    }
    save_plans(&plans)
}

// ── 悬浮球窗口位置（config.json 的 floatWindow 字段，逻辑像素）──

/// 读悬浮球上次位置（逻辑像素 x, y）；不存在 / 损坏返回 None。
pub fn load_float_position() -> Option<(f64, f64)> {
    let v = read_config_value();
    let obj = v.get("floatWindow")?.as_object()?;
    let x = obj.get("x")?.as_f64()?;
    let y = obj.get("y")?.as_f64()?;
    Some((x, y))
}

/// 写悬浮球位置：读-改-写（保留其他字段）+ 原子 tmp+rename。
pub fn save_float_position(x: f64, y: f64) -> std::io::Result<()> {
    write_config_value(
        "floatWindow",
        serde_json::json!({ "x": x, "y": y }),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        legacy_visible_ids, normalize_plans, new_plan_id, seed_plans, strip_with, PlanConfig,
    };

    fn plan(id: &str, vendor: &str) -> PlanConfig {
        PlanConfig {
            id: id.into(),
            vendor: vendor.into(),
            label: String::new(),
            key: "k".into(),
            cookie: String::new(),
            visible: true,
            builtin: None,
        }
    }

    #[test]
    fn normalize_drops_unknown_vendor_and_trims() {
        let out = normalize_plans(vec![
            plan(" GLM-1 ", " GLM "),
            plan("x", "openai"),
            {
                let mut p = plan("mm", "minimax");
                p.label = " 主号 ".into();
                p.key = " k2 ".into();
                p
            },
        ]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].vendor, "glm");
        assert_eq!(out[0].id, "GLM-1");
        assert_eq!(out[1].label, "主号");
        assert_eq!(out[1].key, "k2");
    }

    #[test]
    fn normalize_clears_key_for_builtin_instances() {
        let mut p = plan("glm-b0", "glm");
        p.builtin = Some(0);
        p.key = "误存的明文".into();
        let out = normalize_plans(vec![p]);
        assert_eq!(out[0].key, "", "内置实例 key 必须为空（凭据只走编译期密文）");
        assert_eq!(out[0].builtin, Some(0));
    }

    #[test]
    fn normalize_regenerates_empty_and_duplicate_ids() {
        let out = normalize_plans(vec![plan("", "glm"), plan("dup", "glm"), plan("dup", "glm")]);
        assert_eq!(out.len(), 3);
        assert!(!out[0].id.is_empty(), "空 id 补生成");
        assert_eq!(out[1].id, "dup", "首个 dup 正常保留");
        assert_ne!(out[2].id, "dup", "重复 id 重新生成");
        assert!(out[2].id.starts_with("glm-"));
    }

    #[test]
    fn new_plan_id_has_vendor_prefix_and_unique_enough() {
        let a = new_plan_id("glm");
        let b = new_plan_id("glm");
        assert!(a.starts_with("glm-"));
        assert_ne!(a, b, "连续两次生成不应相同");
    }

    #[test]
    fn seed_creates_builtin_refs_per_key() {
        let all = vec!["glm".to_string(), "minimax".into(), "mimo".into(), "deepseek".into()];
        let plans = seed_plans(&["g1".into()], &[], &[], "", &all);
        assert_eq!(plans.len(), 1, "空列表的品牌不建实例");
        assert_eq!(plans[0].id, "glm-b0");
        assert_eq!(plans[0].builtin, Some(0));
        assert_eq!(plans[0].key, "", "内置凭据不落盘");

        let plans = seed_plans(
            &["g1".into(), "g2".into()],
            &["m1".into()],
            &["d1".into(), "d2".into()],
            "ck",
            &all,
        );
        let ids: Vec<&str> = plans.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["glm-b0", "glm-b1", "minimax-b0", "deepseek-b0", "deepseek-b1", "mimo"]
        );
        assert!(plans.iter().all(|p| p.key.is_empty()));
        assert_eq!(plans[5].cookie, "ck");
        assert_eq!(plans[5].builtin, None, "MiMo 无内置概念，Cookie 明文本地");
    }

    #[test]
    fn seed_maps_legacy_visible_ids() {
        let vis = vec!["glm".to_string(), "deepseek".into()];
        let plans = seed_plans(&["g1".into()], &["m1".into()], &["d1".into()], "", &vis);
        let v = |id: &str| plans.iter().find(|p| p.id == id).unwrap().visible;
        assert!(v("glm-b0"));
        assert!(!v("minimax-b0"));
        assert!(v("deepseek-b0"));
    }

    #[test]
    fn seed_empty_visible_ids_means_all_visible() {
        let plans = seed_plans(&["g1".into()], &["m1".into()], &[], "", &[]);
        assert!(plans.iter().all(|p| p.visible));
    }

    #[test]
    fn legacy_visible_missing_defaults_to_all() {
        let root = serde_json::json!({});
        assert_eq!(
            legacy_visible_ids(&root),
            vec!["glm", "minimax", "mimo", "deepseek"]
        );
    }

    #[test]
    fn legacy_visible_filters_and_dedupes() {
        let root = serde_json::json!({ "visiblePlans": ["GLM", "glm", "mimo", "bogus"] });
        assert_eq!(legacy_visible_ids(&root), vec!["glm", "mimo"]);
    }

    #[test]
    fn has_credential_by_vendor() {
        let mut p = plan("a", "glm");
        assert!(p.has_credential());
        p.key = " ".into();
        assert!(!p.has_credential());

        let mut m = plan("b", "mimo");
        m.key = String::new();
        m.cookie = "c".into();
        assert!(m.has_credential(), "mimo 看 cookie 不看 key");
        m.cookie = String::new();
        assert!(!m.has_credential());
    }

    #[test]
    fn strip_converts_builtin_key_copies() {
        let glm = vec!["g1".to_string(), "g2".into()];
        let minimax = vec!["m1".to_string()];
        let deepseek: Vec<String> = vec![];

        // 与内置 key 相同的明文 → 转 builtin 引用 + 清 key
        let mut g1 = plan("glm-x", "glm");
        g1.key = "g1".into();
        let mut g2 = plan("glm-y", "glm");
        g2.key = "g2".into();
        // 非内置品牌 / 已是 builtin / 未知品牌 / mimo 不动
        let mut other = plan("ds-1", "deepseek");
        other.key = "自己的key".into();
        let mut already = plan("glm-b0", "glm");
        already.key.clear();
        already.builtin = Some(0);
        let mut mimo = plan("mimo", "mimo");
        mimo.key.clear();
        mimo.cookie = "c".into();

        let (out, changed) = strip_with(vec![g1, g2, other, already, mimo], &glm, &minimax, &deepseek);
        assert!(changed);
        assert_eq!(out[0].builtin, Some(0));
        assert_eq!(out[0].key, "");
        assert_eq!(out[1].builtin, Some(1));
        assert_eq!(out[1].key, "");
        assert_eq!(out[2].key, "自己的key", "非内置 key 不动");
        assert_eq!(out[2].builtin, None);
        assert_eq!(out[3].builtin, Some(0), "已是 builtin 不动");
        assert_eq!(out[4].cookie, "c", "mimo 不动");
    }

    #[test]
    fn strip_no_change_when_no_copies() {
        let (out, changed) = strip_with(vec![plan("glm-x", "glm")], &["g1".into()], &[], &[]);
        assert!(!changed);
        assert_eq!(out[0].key, "k");
    }
}
