# legacy — 旧版 Python/PySide6 实现（已归档）

本目录是项目重构为 **Tauri(Rust) + Svelte** 多平台应用之前的原始实现，
仅作为**行为规格参考**保留，不再维护、不再运行。

## 为什么留着

重写时，三个 fetcher 的 API 解析逻辑、数据合并规则、占位项策略、
防抖窗口等行为细节都**逐字段对照**过这里的 Python 源码，以保证
新版功能与体验一致。需要核对某个具体行为时来这里查。

## 目录对应关系（legacy → 新版）

| legacy 文件 | 新版对应 | 用途 |
|-------------|---------|------|
| `data_manager.py` | `src-tauri/src/data.rs` + `watcher.rs` | 数据模型 + 文件监听/防抖 |
| `fetch_worker.py` | `src-tauri/src/main.rs`（tokio 并发） | 后台并行拉取 |
| `deepseek_fetcher.py` | `src-tauri/src/fetcher/deepseek.rs` | DeepSeek 余额 |
| `glm_fetcher.py` | `src-tauri/src/fetcher/glm.rs` | GLM 配额 |
| `minimax_fetcher.py` | `src-tauri/src/fetcher/minimax.rs` | MiniMax 配额 |
| `paths.py` | Tauri `app_data_dir` / `resource_dir` | 路径分流 |
| `widget.py` | `src/`（Svelte 前端）+ `window.rs` | UI + 平台特效 |
| `main.py` | `src-tauri/src/main.rs` | 入口 |
| `balance.json` | `src-tauri/resources/balance.json` | 首次运行模板 |
| `config.json`(运行期生成) | Tauri app data: `config.json` | API 密钥 |

## 技术栈

- Python 3.10+ · PySide6 · Windows DWM acrylic
- 仅支持 Windows（毛玻璃走 ctypes.windll 调 DWM/user32）

新版（仓库根目录）改用 Tauri + Svelte，支持 Windows + macOS。
