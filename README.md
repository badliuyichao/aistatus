# AI API 余额监控悬浮框

> 跨平台桌面悬浮框，实时监控 AI 服务（GLM 智谱AI、DeepSeek、MiniMax 稀宇科技）的套餐限额和充值余额。

基于 **Tauri 2（Rust）+ Svelte 5** 重写，支持 **Windows + macOS**。

## 功能

- 📊 **桌面悬浮显示** — 无边框、置顶、毛玻璃效果（Win acrylic/mica、mac vibrancy）
- 🔄 **自动刷新** — 编辑 `balance.json` 后自动更新；启动后每 60 秒定时拉取 API 数据
- 🎨 **双模式卡片**
  - **配额型** — 已用/总量 + 进度条（GLM 5小时/周限额、MiniMax 同）
  - **余额型** — 剩余金额（DeepSeek 充值余额）
- 🖼️ **品牌图标** — 内置 GLM / DeepSeek / MiniMax Logo
- ⚡ **非阻塞抓取** — 三个 API 在后台 `tokio::join!` 并发请求，刷新期间界面不卡顿
- 🖱️ **可拖拽 / 可缩放** — 拖拽标题栏移动；无边框窗口原生支持缩放
- 🖥️ **系统托盘** — 最小化到托盘
- ⌨️ **快捷键** — `F5` 刷新数据
- ⚙ **API Key 配置** — 首次运行自动弹窗引导，密钥仅存 Rust 侧（不进 webview DOM）

## 架构

```
前端 (Svelte 5 + TS)          Rust 后端 (Tauri 2)
─────────────────────         ──────────────────────
纯渲染，零网络/零文件IO        所有 I/O 与业务逻辑
                               · 三 fetcher (reqwest)
invoke() ──────────────────→   · config.json 密钥
        └────────────────────   · balance.json 监听+防抖
listen('services-updated')     · 60s 定时刷新 (tokio)
                               · 原生毛玻璃/托盘/置顶
```

**边界原则**：API 密钥与网络请求**只在 Rust 侧**（规避 CORS、密钥不进前端内存）。

## 目录结构

```
├── src/                       前端 (Svelte)
│   ├── App.svelte             主壳：标题栏 + 卡片 + 页脚 + 右键菜单
│   ├── components/            TitleBar / ServiceCard / QuotaRow / BalanceView / ConfigDialog
│   ├── stores/services.ts     服务数据响应式状态
│   ├── api.ts                 Tauri IPC 封装 (invoke + listen)
│   ├── types.ts               与 Rust 对齐的 TS 类型
│   └── assets/                品牌 Logo
├── src-tauri/                 后端 (Rust)
│   ├── src/
│   │   ├── data.rs            数据模型 (对齐 legacy Python)
│   │   ├── config.rs          config.json 读写 (跨平台路径)
│   │   ├── fetcher/           deepseek / glm / minimax 抓取器
│   │   ├── merge.rs           三家结果合并进 BalanceData
│   │   ├── commands.rs        暴露给前端的 #[tauri::command]
│   │   ├── state.rs           共享状态 + 后台拉取调度
│   │   └── lib.rs             入口：建窗口 + 定时 + 引导
│   ├── tauri.conf.json        窗口/包名/打包配置
│   └── resources/balance.json 首次运行模板
├── legacy/                    旧版 Python/PySide6 实现（行为参考，已归档）
└── package.json
```

## 开发

### 前置要求

- **Rust**（stable）+ cargo
- **Node.js** 20+
- **pnpm**（`corepack enable`）
- **macOS**：Xcode Command Line Tools
- **Windows**：MSVC build tools + WebView2

### 本地运行

```bash
pnpm install
pnpm tauri dev
```

首次会编译大量 Rust crate（数分钟），后续增量很快。

### 类型检查

```bash
pnpm check          # 前端 svelte-check
cd src-tauri && cargo check   # 后端类型检查
```

## 配置 API Key

首次运行会自动弹出「API Key 配置」对话框（也可随时点标题栏 ⚙ 按钮重开）。密钥存放在跨平台用户配置目录：

| 平台 | 路径 |
|------|------|
| macOS | `~/Library/Application Support/aistatus/config.json` |
| Windows | `%APPDATA%\aistatus\config.json` |

```json
{
  "deepseekApiKey": "sk-...",
  "glmApiKey": "...",
  "minimaxApiKey": "..."
}
```

| 服务 | 类型 | 获取方式 |
|------|------|----------|
| GLM 智谱AI Coding Plan | quota | 智谱开放平台 → API 密钥 |
| DeepSeek | balance | DeepSeek 开放平台 → API 密钥 |
| MiniMax 稀宇科技 Coding Plan | quota | MiniMax 用户中心 → API Key |

## 打包分发

```bash
pnpm tauri build
```

- **macOS**：产出 `.app` + `.dmg`（`src-tauri/target/release/bundle/`）
- **Windows**：产出 MSI / NSIS 安装包

> ⚠ 未签名的 macOS 应用首次打开需右键 → 打开。

## 技术栈

- **Rust** + **Tauri 2**（后端 / 原生集成）
- **Svelte 5** + **TypeScript** + **Vite**（前端）
- **reqwest / tokio**（异步 HTTP）
- **window-vibrancy**（原生毛玻璃）

## legacy

`legacy/` 目录是重构前的 Python/PySide6 实现（仅 Windows），保留作为**行为规格参考**。三个 fetcher 的解析逻辑、合并规则等都逐字段对照过它。详见 `legacy/README-legacy.md`。
