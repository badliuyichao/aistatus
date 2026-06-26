//! 三个 API 抓取器 —— 对应 legacy `deepseek_fetcher.py` / `glm_fetcher.py` /
//! `minimax_fetcher.py`。
//!
//! 每个 fetcher：
//!   - 从 config.json 拿自己的 key（`has_key` 判断）
//!   - 异步 HTTP 请求（`reqwest`，10s 超时，对应 Python urllib timeout=10）
//!   - 解析官方响应 → 转成展示用的 ServiceInfo/QuotaItem
//!   - 失败返回 None（对应 Python 的 return None）
//!
//! 三家请求由 `main.rs` 用 `futures::join!` 并发执行（对应 Python
//! `ThreadPoolExecutor(max_workers=3)`）。

pub mod deepseek;
pub mod glm;
pub mod minimax;

use std::time::Duration;

/// 构建一个统一的 reqwest 客户端：10s 超时 + rustls。
/// 对应 Python 各 fetcher 的 `urllib.request.urlopen(req, timeout=10)`。
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .gzip(true)
        .build()
        .expect("reqwest client build failed")
}
