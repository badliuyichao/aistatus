//! 应用共享状态 + 后台拉取调度。
//!
//! 集中管理：当前 API key、当前展示数据快照、并发拉取与防重入。
//!
//! 拉取流程（每次 request_refresh）：
//!   1. 若已有拉取在跑 → 直接返回
//!   2. 并发拉三家（`futures::join`）
//!   3. 读 balance.json → merge → 更新快照
//!   4. emit `services-updated` 给前端

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::config;
use crate::data::{BalanceData, FetchOutcome};
use crate::fetcher;
use crate::merge::merge_api_into;
use crate::secrets;

/// 编译进二进制的首次启动模板，不依赖安装目录或开发目录中的外部文件。
const BALANCE_TEMPLATE: &str = include_str!("../resources/balance.json");

pub struct AppState {
    /// 当前展示数据快照（get_services 返回它）。
    snapshot: tokio::sync::Mutex<BalanceData>,
    /// 防重入：正在拉取时为 true，新请求被丢弃。
    /// Arc 便于 BusyGuard 跨 await/panic 边界持有，Drop 时复位。
    busy: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        // 启动时从 balance.json 读骨架，两家标记 loading。
        let mut data = load_balance_or_default();
        for svc in &mut data.services {
            svc.loading = true;
        }
        data.timestamp = Some(BalanceData::now_timestamp());

        Self {
            snapshot: tokio::sync::Mutex::new(data),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 返回当前快照（给前端初始数据）。
    pub async fn snapshot(&self) -> BalanceData {
        self.snapshot.lock().await.clone()
    }

    /// 触发一次后台拉取。不阻塞调用方；若已在拉取则丢弃（防重入）。
    ///
    /// 关联函数（不接 &self）：调用方只需传 AppHandle，内部通过
    /// `app.state::<AppState>()` 取状态——这样调用处不存在 borrow/move
    /// 冲突，`handle` 可以直接 move 进 async block。
    ///
    /// busy 复位用 BusyGuard（Drop），即使 do_fetch_and_emit panic 也会复位，
    /// 避免 busy 永久卡 true 导致常驻后台进程后续刷新全部静默失效。
    pub async fn request_refresh(app: AppHandle) {
        let state = app.state::<AppState>();
        // 防重入
        if state.busy.swap(true, Ordering::SeqCst) {
            return;
        }

        // 克隆 busy 的 Arc 到 guard，使其在 spawn 的 future 内独立持有
        let busy = state.busy.clone();
        tokio::spawn(async move {
            // guard 在 Drop 时复位 busy；即使下方 panic 也保证复位。
            // 放在 future 顶部，覆盖整个 do_fetch_and_emit 作用域。
            let _guard = BusyGuard(busy);
            let state = app.state::<AppState>();
            do_fetch_and_emit(state.inner(), &app).await;
        });
    }
}

/// busy 标志的 RAII 守卫：Drop 时把 busy 复位为 false。
/// 保证即使被守卫的作用域内 panic，busy 也不会永久卡死（对常驻后台进程至关重要）。
struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// 一次完整拉取：并发三请求 → merge → 更新快照 → emit。
///
/// GLM/MiniMax 的 key 由 secrets 模块运行期解密提供（编译期加密注入）；
/// MiMo 的 Cookie 从 config.json 现读（用户运行期可改，60s 一轮的读文件
/// 开销可忽略，天然拿到刚保存的新值，无需缓存失效机制）。
async fn do_fetch_and_emit(state: &AppState, app: &AppHandle) {
    // 并发拉三家。key/cookie 在此处临时取用，用完即弃。
    // join! 宏支持任意元数（join 函数只重载到二元）；凭据需先绑定，
    // 临时 String 内联进宏会被判跨 await 悬垂借用（宏展开为多条子语句，
    // 不享受函数调用的临时值生命周期延长）。
    let glm_key = secrets::glm_key();
    let minimax_key = secrets::minimax_key();
    let mimo_cookie = config::load_mimo_cookie();
    let (glm, mm, mimo) = futures::join!(
        fetcher::glm::fetch(&glm_key),
        fetcher::minimax::fetch(&minimax_key),
        fetcher::mimo::fetch(&mimo_cookie),
    );

    let outcome = FetchOutcome {
        glm,
        minimax: mm,
        mimo,
    };

    // 每次刷新都重新读用户可编辑的 balance.json。
    let mut data = load_balance_or_default();
    merge_api_into(&mut data, &outcome);
    data.timestamp = Some(BalanceData::now_timestamp());

    // 更新快照
    *state.snapshot.lock().await = data.clone();

    // 悬浮条高度按 MiMo 卡片有无自适应（在 emit 前取数据，emit 会 move）
    let has_mimo = data.services.iter().any(|s| s.name.starts_with("MiMo"));

    // 广播给前端
    let _ = app.emit("services-updated", data);

    fit_float_window(app, has_mimo);
}

/// 悬浮条尺寸：与 tauri.conf.json 的 float 窗口保持一致；MiMo 多一行。
const FLOAT_WIDTH: f64 = 180.0;
const FLOAT_HEIGHT: f64 = 60.0;
const FLOAT_HEIGHT_MIMO: f64 = 80.0;

/// 悬浮条高度自适应：MiMo 卡片（可选服务）出现/消失时增减一行（60 ↔ 80）。
///
/// 仅高度实际变化时才动窗口。用户没拖过（无保存位置）时重新锚回平台默认角，
/// 避免增高后底部压到任务栏（Win/Linux 底部锚定）；mac 顶部锚定，
/// set_size 保持左上角不动即可，无需重锚。
fn fit_float_window(app: &AppHandle, has_mimo: bool) {
    let Some(win) = app.get_webview_window("float") else {
        return;
    };
    let target = if has_mimo { FLOAT_HEIGHT_MIMO } else { FLOAT_HEIGHT };
    let scale = win.scale_factor().unwrap_or(1.0);
    let cur_h = win
        .outer_size()
        .map(|s| s.height as f64 / scale)
        .unwrap_or(0.0);
    if (cur_h - target).abs() < 0.5 {
        return;
    }
    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        FLOAT_WIDTH, target,
    )));
    if config::load_float_position().is_none() {
        #[cfg(target_os = "macos")]
        {
            crate::anchor_top_right(&win);
        }
        #[cfg(not(target_os = "macos"))]
        {
            crate::anchor_to_bottom_right(&win);
        }
    }
}

/// 读 balance.json；文件缺失时从内嵌模板创建，解析失败时返回默认值。
pub fn load_balance_or_default() -> BalanceData {
    let path = config::balance_path();
    match std::fs::read_to_string(&path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(_) => {
            // 首次运行：从编译进程序的模板创建文件，再读。
            let _ = seed_balance_template();
            config::read_json_or_default::<BalanceData>(&path)
        }
    }
}

/// 首次运行：将编译进程序的 balance.json 模板写入用户数据目录。
fn seed_balance_template() -> std::io::Result<()> {
    let dest = config::balance_path();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, BALANCE_TEMPLATE)
}
