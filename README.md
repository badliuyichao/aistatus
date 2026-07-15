# AI API 余额监控

> 跨平台常驻托盘应用，实时监控 AI 服务（GLM 智谱AI、DeepSeek、MiniMax 稀宇科技）的套餐限额和充值余额。

基于 **Tauri 2（Rust）+ Svelte 5**，支持 **Windows + macOS**。

## 功能

- 🖥️ **常驻系统托盘** — 启动后驻留托盘（macOS 不显示 Dock 图标，Windows 不进任务栏），点击托盘图标弹出菜单
- 🧭 **窗口唤起** — 托盘菜单「打开主界面」唤起悬浮窗；macOS 上窗口跟随托盘图标位置弹出
- 📊 **悬浮展示** — 置顶、无边框、毛玻璃（Win acrylic/mica、mac vibrancy）
- 🔄 **自动刷新** — 启动后台拉取一次，之后每 60 秒定时刷新；随时 `F5` 手动刷新
- 🎨 **双模式卡片**
  - **配额型** — 已用/总量 + 进度条（GLM 5小时/周限额、MiniMax 同）
  - **余额型** — 剩余金额（DeepSeek 充值余额）
- 🖼️ **品牌图标** — 内置 GLM / DeepSeek / MiniMax Logo
- ⚡ **非阻塞抓取** — 三个 API 在后台 `tokio::join!` 并发请求，刷新期间界面不卡顿
- 🔴 **原生窗口控制** — macOS 红绿灯（关闭/最小化/全屏），关闭按钮=隐藏窗口（保留进程）
- 🖱️ **右键菜单** — 刷新数据 / 编辑数据文件 / 配置 API Key
- ⚙ **API Key 配置** — 托盘「设置…」或右键「配置 API Key」打开对话框；密钥仅存 Rust 侧（不进 webview DOM）

## 退出与关闭

- **关闭窗口**（macOS 红绿灯红色 / Windows 关闭）：仅隐藏窗口，进程驻留托盘继续后台刷新
- **真正退出**：只能通过托盘菜单「退出」

## 架构

```
前端 (Svelte 5 + TS)          Rust 后端 (Tauri 2)
─────────────────────         ──────────────────────
纯渲染，零网络/零文件IO        所有 I/O 与业务逻辑
                               · 三 fetcher (reqwest)
invoke() ──────────────────→   · config.json 密钥
        └────────────────────   · 60s 定时刷新 (tokio)
listen('services-updated')     · balance.json 模板
listen('open-config')          · 原生毛玻璃/托盘/窗口定位
                               · macOS 红绿灯关闭拦截
```

**边界原则**：API 密钥与网络请求**只在 Rust 侧**（规避 CORS、密钥不进前端内存）。

## 目录结构

```
├── src/                       前端 (Svelte)
│   ├── App.svelte             主壳：标题区 + 卡片 + 页脚 + 右键菜单 + 托盘事件
│   ├── components/            ServiceCard / QuotaRow / BalanceView / ConfigDialog
│   ├── stores/services.ts     服务数据响应式状态
│   ├── api.ts                 Tauri IPC 封装 (invoke + listen)
│   ├── types.ts               与 Rust 对齐的 TS 类型
│   └── assets/                品牌 Logo
├── src-tauri/                 后端 (Rust)
│   ├── src/
│   │   ├── data.rs            数据模型
│   │   ├── config.rs          config.json 读写 (跨平台路径)
│   │   ├── fetcher/           deepseek / glm / minimax 抓取器
│   │   ├── merge.rs           三家结果合并进 BalanceData
│   │   ├── commands.rs        暴露给前端的 #[tauri::command]
│   │   ├── state.rs           共享状态 + 后台拉取调度（防重入 RAII guard）
│   │   ├── backdrop.rs        原生毛玻璃（mac vibrancy / Win acrylic）
│   │   └── lib.rs             入口：托盘 + 窗口定位 + 定时 + 关闭拦截
│   ├── tauri.conf.json        窗口/包名/打包配置
│   └── resources/balance.json 首次运行模板
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

> 开发模式下启动后**窗口默认隐藏**，点击菜单栏/托盘图标 → 「打开主界面」唤起（与生产环境行为一致）。

### 类型检查

```bash
pnpm check                              # 前端 svelte-check
cargo check --manifest-path src-tauri/Cargo.toml   # 后端类型检查
```

## 配置 API Key

首次唤起主界面时若未配置 key 会自动弹出对话框；之后随时通过**托盘菜单「设置…」**或**右键菜单「配置 API Key」**重开。密钥存放在跨平台用户配置目录：

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
