//! 三个服务商 API 抓取器。
//!
//! 每个 fetcher：
//!   - 从 config.json 拿自己的 key（`has_key` 判断）
//!   - 异步 HTTP 请求（`reqwest`，10s 超时）
//!   - 解析官方响应 → 转成展示用的 ServiceInfo/QuotaItem
//!   - 失败返回 None
//!
//! 三家请求由共享状态模块用 `futures::join!` 并发执行。

pub mod deepseek;
pub mod glm;
pub mod minimax;

use std::sync::OnceLock;
use std::time::Duration;

/// 全局共享的 reqwest 客户端（连接池复用，避免每次刷新重建 3 个 client）。
///
/// 用 OnceLock 实现进程级单例：
///   - 复用 TLS 会话 / keep-alive 连接，减少重复握手
///   - builder 在常驻后台进程里只会构造一次，失败时进程直接退出（启动期报错
///     优于运行期 panic；OnceLock::get_or_init 里的 expect 只在首次调用执行）
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn http_client() -> reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .gzip(true)
                .build()
                // 启动期构造失败属环境异常（如 rustls 不可用），直接 panic
                // 让进程暴露问题；后续运行期不再构造，无 panic 风险。
                .expect("reqwest client build failed")
        })
        .clone()
}
