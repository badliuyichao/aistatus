//! 应用共享状态 + 后台拉取调度。
//!
//! 集中管理：当前 API key、当前展示数据快照、并发拉取与防重入。
//!
//! 拉取流程（每次 request_refresh）：
//!   1. 若已有拉取在跑 → 直接返回
//!   2. 并发拉三家（`tokio::join!`）
//!   3. 读 balance.json → merge → 更新快照
//!   4. emit `services-updated` 给前端

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::future::join3;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{self, ApiKeys};
use crate::data::{BalanceData, FetchOutcome};
use crate::fetcher;
use crate::merge::merge_api_into;

/// 编译进二进制的首次启动模板，不依赖安装目录或开发目录中的外部文件。
const BALANCE_TEMPLATE: &str = include_str!("../resources/balance.json");

pub struct AppState {
    /// 当前 key 副本（save_keys 时更新；拉取时读，避免每次读盘）。
    keys: tokio::sync::Mutex<ApiKeys>,
    /// 当前展示数据快照（get_services 返回它）。
    snapshot: tokio::sync::Mutex<BalanceData>,
    /// 防重入：正在拉取时为 true，新请求被丢弃。
    /// Arc 便于 BusyGuard 跨 await/panic 边界持有，Drop 时复位。
    busy: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(initial_keys: ApiKeys) -> Self {
        // 启动时从 balance.json 读骨架，三家标记 loading。
        let mut data = load_balance_or_default();
        for svc in &mut data.services {
            svc.loading = true;
        }
        data.timestamp = Some(BalanceData::now_timestamp());

        Self {
            keys: tokio::sync::Mutex::new(initial_keys),
            snapshot: tokio::sync::Mutex::new(data),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 返回当前快照（给前端初始数据）。
    pub async fn snapshot(&self) -> BalanceData {
        self.snapshot.lock().await.clone()
    }

    /// 更新运行期 key 副本。
    pub async fn update_keys(&self, keys: ApiKeys) {
        *self.keys.lock().await = keys;
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

        let keys = state.keys.lock().await.clone();
        // 克隆 busy 的 Arc 到 guard，使其在 spawn 的 future 内独立持有
        let busy = state.busy.clone();
        tokio::spawn(async move {
            // guard 在 Drop 时复位 busy；即使下方 panic 也保证复位。
            // 放在 future 顶部，覆盖整个 do_fetch_and_emit 作用域。
            let _guard = BusyGuard(busy);
            let state = app.state::<AppState>();
            do_fetch_and_emit(state.inner(), &keys, &app).await;
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
async fn do_fetch_and_emit(state: &AppState, keys: &ApiKeys, app: &AppHandle) {
    // 并发拉三家。
    let (ds, glm, mm) = join3(
        fetcher::deepseek::fetch(&keys.deepseek_api_key),
        fetcher::glm::fetch(&keys.glm_api_key),
        fetcher::minimax::fetch(&keys.minimax_api_key),
    )
    .await;

    let outcome = FetchOutcome {
        deepseek: ds,
        glm,
        minimax: mm,
    };

    // 每次刷新都重新读用户可编辑的 balance.json。
    let mut data = load_balance_or_default();
    merge_api_into(&mut data, &outcome);
    data.timestamp = Some(BalanceData::now_timestamp());

    // 更新快照
    *state.snapshot.lock().await = data.clone();

    // 广播给前端
    let _ = app.emit("services-updated", data);
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
