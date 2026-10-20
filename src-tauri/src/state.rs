//! 应用共享状态 + 后台拉取调度。
//!
//! 集中管理：当前 plan 实例配置、当前展示数据快照、并发拉取与防重入。
//!
//! 拉取流程（每次 request_refresh）：
//!   1. 若已有拉取在跑 → 直接返回
//!   2. 现读 config.json 的 plans → 按实例并发拉取（join_all）
//!   3. build_snapshot 组装快照 → 更新状态
//!   4. 出口按 plan.visible 过滤后 emit `services-updated` 给前端

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::config;
use crate::data::BalanceData;
use crate::snapshot;

pub struct AppState {
    /// 当前展示数据快照（get_services 返回它）。
    snapshot: tokio::sync::Mutex<BalanceData>,
    /// 防重入：正在拉取时为 true，新请求被丢弃。
    /// Arc 便于 BusyGuard 跨 await/panic 边界持有，Drop 时复位。
    busy: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        // 启动初始快照：已配置凭据的实例给 loading 卡片，等首轮拉取填充。
        // 首次升级迁移（内置 key 播种 plans）在 load_plans 内完成。
        let plans = config::load_plans();
        let data = snapshot::initial_snapshot(&plans);

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

/// 一次完整拉取：按 plans 并发拉全部实例 → 组装快照 → emit。
///
/// plans 每轮现读 config.json（用户在设置里增删实例 / 换 key 后立即生效，
/// 无缓存失效机制）；每个实例一次 fetch（空凭据的实例由 fetcher 返回
/// Skipped 跳过，不出卡片）。
async fn do_fetch_and_emit(state: &AppState, app: &AppHandle) {
    let plans = config::load_plans();

    // 并发拉取全部实例。join_all 的 future 借用 plans，作用域内 await 安全。
    let results = futures::future::join_all(plans.iter().map(snapshot::fetch_plan)).await;

    let mut data = snapshot::build_snapshot(&plans, results);

    // 快照保留全量合并结果；可见性是展示层配置（每实例的 visible），
    // 只在出口（此处 emit / get_services）过滤，取消隐藏可立即用快照缓存回显。
    *state.snapshot.lock().await = data.clone();

    snapshot::filter_visible(&mut data, &plans);

    // 悬浮条高度按可见行数自适应（在 emit 前取数据，emit 会 move）
    let rows = data.services.len();

    // 广播给前端
    let _ = app.emit("services-updated", data);

    fit_float_window(app, rows);
}

/// 按当前可见性配置把快照重新推给前端：显隐 / 实例增删配置保存后立即生效，
/// 不等下一轮拉取（60s / 手动刷新）。快照是全量的，过滤在此进行——
/// 取消隐藏时用缓存数据即时回显，随后的 request_refresh 再补新数据。
pub(crate) async fn reemit_visible_snapshot(app: &AppHandle) {
    let state = app.state::<AppState>();
    let plans = config::load_plans();
    let mut view = state.snapshot().await;
    snapshot::filter_visible(&mut view, &plans);
    let rows = view.services.len();
    let _ = app.emit("services-updated", view);
    fit_float_window(app, rows);
}

/// 悬浮条尺寸：与 tauri.conf.json 的 float 窗口保持一致。
/// 宽 220：品牌列 76 + 进度条 ~118（barColors 列加宽后仍保有足够条长）。
/// 高度 = 基础 + 每可见服务一行（20 逻辑像素）：
/// 1 行 40 / 2 行 60 / 3 行 80 / 4 行 100（多实例时按实际行数增长）。
const FLOAT_WIDTH: f64 = 220.0;
const FLOAT_ROW_H: f64 = 20.0;
const FLOAT_BASE_H: f64 = 20.0;

/// 悬浮条尺寸自适应：可见服务行数变化时增减高度；宽度与 FLOAT_WIDTH 不符时
/// （如升级改宽）一并修正。
///
/// 用户没拖过（无保存位置）时重新锚回平台默认角，避免增高后底部压到任务栏
/// （Win/Linux 底部锚定）；mac 顶部锚定，set_size 保持左上角不动即可，无需重锚。
/// 保存过位置时 set_size 以左上角为基准向右生长，贴屏幕右缘摆放的窗口加宽后
/// 会被推出屏幕——把 x 左移宽度增量补偿，保持原右缘位置。
fn fit_float_window(app: &AppHandle, rows: usize) {
    let Some(win) = app.get_webview_window("float") else {
        return;
    };
    let rows = rows.max(1); // 防御：配置保证至少保留一个实例，异常数据时不缩成 0 高
    let target = FLOAT_BASE_H + FLOAT_ROW_H * rows as f64;
    let scale = win.scale_factor().unwrap_or(1.0);
    let (cur_w, cur_h) = match win.outer_size() {
        Ok(s) => (s.width as f64 / scale, s.height as f64 / scale),
        Err(_) => (FLOAT_WIDTH, target),
    };
    let width_changed = (cur_w - FLOAT_WIDTH).abs() > 0.5;
    if !width_changed && (cur_h - target).abs() < 0.5 {
        return;
    }
    let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        FLOAT_WIDTH, target,
    )));

    // 保存过位置（不会走下方重锚）且窗口加宽：贴右缘时左移补偿，防止出屏
    if width_changed && config::load_float_position().is_some() {
        if let (Ok(pos), Ok(Some(mon))) = (win.outer_position(), win.primary_monitor()) {
            let grow = ((FLOAT_WIDTH - cur_w) * scale) as i32;
            let mon_right = mon.position().x + mon.size().width as i32;
            let new_right = pos.x + (FLOAT_WIDTH * scale) as i32;
            if new_right > mon_right {
                let _ = win.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(pos.x - grow, pos.y),
                ));
            }
        }
    }

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
