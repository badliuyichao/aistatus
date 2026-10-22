# AI API 余额监控

> 跨平台常驻托盘应用，实时监控 AI 服务的 Coding Plan 用量与充值余额：
> **GLM 智谱AI / MiniMax / 小米 MiMo / DeepSeek**。

基于 **Tauri 2（Rust）+ Svelte 5 + TypeScript + Vite**，支持 **Windows + macOS**（Linux 次要）。

> 开发规范与实现细节的唯一来源是 [AGENTS.md](./AGENTS.md)，本文件只做概览。

## 功能

- 🖥️ **常驻系统托盘** — 启动后驻留托盘（macOS 不显示 Dock 图标，Windows 不进任务栏），菜单：打开主界面 / 悬浮条开关 / 设置… / 退出
- 📊 **主看板** — 每条 Coding Plan 实例一张卡片，窗口高度随内容自适应；原生毛玻璃（Win acrylic/mica、mac vibrancy），失败降级 CSS；深色 / 浅色 / 跟随系统三档主题，跨窗口同步
- 🎈 **悬浮条** — 220px 宽置顶小条，每实例一行：配额画进度条（GLM / MiniMax 的 5 小时 + 周窗口上下双条），DeepSeek 显示余额金额；高度随可见行数自适应，位置可拖拽并记忆
- 🧩 **Coding Plan 多实例** — 同品牌可配多份（如 2 个 GLM plan、2 个 DeepSeek key），实例在设置对话框增删 / 改名 / 换 key / 显隐 / 排序；主看板与悬浮条同序渲染
- 🔄 **自动刷新** — 启动拉取一次，之后每 60s 定时刷新；`F5` 或右键「刷新数据」手动触发；plans 配置每轮现读文件，增删 / 换 key / 改 Cookie 即时生效，无需重启
- 🎨 **双模式卡片** — **配额型**：已用/总量 + 进度条（按品牌 + 周期分色，GLM 5 小时条为淡蓝→深红固定色带，均带重置倒计时）；**余额型**（DeepSeek）：大号金额 + 充值/赠送明细 + 阈值状态标签（≥¥50 安全 / ¥10–50 预警 / <¥10 危险）
- 🖼️ **品牌图标** — 来自 [Lobe Icons](https://github.com/lobehub/lobe-icons)（GLM / MiniMax / MiMo / DeepSeek）
- 🔒 **凭据不进前端** — 网络请求与密钥只在 Rust 侧（规避 CORS、不进 webview）；「内置实例」的 key 仅以密文编译进二进制、永不落盘
- 🍎 **mac 原生行为** — 红绿灯关闭=隐藏窗口（进程驻留托盘继续刷新），仅托盘「退出」结束进程；窗口跟随托盘图标弹出

## 监控的服务

| 服务 | 类型 | 凭据 | Key 来源 |
|------|------|------|----------|
| GLM 智谱AI Coding Plan | 配额（5小时 / 周窗口） | API Key | 智谱开放平台 → API 密钥 |
| MiniMax Coding Plan | 配额（5小时 / 周窗口） | API Key | MiniMax 用户中心 → API Key |
| 小米 MiMo Token Plan | 配额（周期套餐） | 浏览器 Cookie | platform.xiaomimimo.com 登录后复制（逆向接口，非官方公开 API） |
| DeepSeek | 余额（prepaid 按量计费，无进度条语义） | API Key | DeepSeek 开放平台 → API 密钥 |

认证失败 / 账号无 plan / Cookie 失效均显示明确占位提示（如「API Key 无效 · 请在设置中检查」），不显示误导性 N/A。

## 架构

```
前端 (Svelte 5 + TS)                Rust 后端 (Tauri 2)
─────────────────────               ──────────────────────
纯渲染，零网络/零文件 IO             所有 I/O 与业务逻辑
                                    · fetcher/ 四家抓取器（reqwest）
invoke(get_services) ────────────→  · config.json 读写（plans 实例列表）
listen('services-updated') ←──────  · 按实例并发拉取 → 组装快照 → 出口按 visible 过滤
listen('open-config'/'theme-changed') · 60s 定时刷新（tokio，防重入）
                                    · 原生毛玻璃 / 托盘 / 窗口定位 / 悬浮条尺寸自适应
```

**边界原则**：API Key / Cookie 与网络请求只在 Rust 侧，凭据不进前端内存。

## 目录结构

```
├── src/                        前端 (Svelte 5)
│   ├── App.svelte              主壳：标题栏 + 卡片列表 + 页脚 + 右键菜单
│   ├── components/             ServiceCard / ConfigDialog / TitleBar / BalanceView / QuotaRow / FloatWidget
│   ├── barColors.ts            进度条配色（vendor + 周期分色，看板与悬浮条共用）
│   ├── balanceStatus.ts        DeepSeek 余额阈值分级（安全 / 预警 / 危险）
│   ├── stores/                 services.svelte.ts（数据）/ theme.svelte.ts（主题）
│   ├── api.ts                  Tauri IPC 封装（invoke + listen）
│   ├── types.ts                与 Rust 对齐的 TS 类型（ServiceInfo / PlanConfig）
│   └── assets/                 品牌 Logo（Lobe Icons 640×640 PNG）
├── src-tauri/                  后端 (Rust)
│   ├── src/
│   │   ├── lib.rs              入口：窗口 / 托盘 / 定位 / 60s 定时刷新
│   │   ├── commands.rs         #[tauri::command]（get_services / get+save_plans / 主题 / 悬浮条 / MiMo Cookie …）
│   │   ├── config.rs           config.json 读写（主题、悬浮条位置、plans 实例 + 播种迁移 + 规范化）
│   │   ├── snapshot.rs         按 plans 并发拉取 + 组装快照 + 可见性过滤
│   │   ├── state.rs            共享状态 + 拉取调度（防重入）+ 悬浮条尺寸自适应
│   │   ├── secrets.rs          内置 key 解密（仅作播种迁移源）
│   │   ├── data.rs             数据模型（ServiceInfo / PlanFetch 三态）
│   │   ├── backdrop.rs         原生毛玻璃（Win acrylic/mica、mac vibrancy）
│   │   └── fetcher/            glm / minimax / mimo / deepseek 抓取器
│   ├── tauri.conf.json         窗口 / bundle / 打包配置
│   └── Cargo.toml              version 由 sync-version 维护，勿手改
└── scripts/                    sync-version / build / encrypt-keys / fetch-icon
```

## 开发

### 前置要求

- **Rust** stable + cargo（**Windows 必须 msvc 工具链**，gnu 产物分发到其他机器会缺 WebView2Loader.dll）
- **Node.js** 20+、**pnpm**（`corepack enable`）
- **macOS**：Xcode Command Line Tools
- **Windows**：Visual Studio Build Tools（MSVC linker）+ WebView2

### 本地运行

```bash
pnpm install
pnpm tauri dev
```

首次编译较慢（数分钟）。启动后窗口默认隐藏，点托盘图标 → 「打开主界面」唤起（与生产行为一致）。

### 常用命令

```bash
pnpm check           # svelte-check 类型检查
pnpm build:tauri     # 打包发布（自动同步版本号；Windows 无条件强制 msvc 工具链）
pnpm sync-version    # 手动同步版本号（build 前会自动跑）
```

## 配置

配置存于跨平台用户配置目录（右键菜单「编辑配置文件」可用系统默认编辑器打开）：

| 平台 | 路径 |
|------|------|
| macOS | `~/Library/Application Support/aistatus/config.json` |
| Windows | `%APPDATA%\aistatus\config.json` |

核心是 `plans` 实例数组（同品牌可多份，显示顺序 = 数组顺序）：

```json
{
  "theme": "system",
  "plans": [
    { "id": "glm-a1b2c3",      "vendor": "glm",      "label": "主账号", "key": "...",     "visible": true },
    { "id": "mimo-d4e5f6",     "vendor": "mimo",     "label": "",       "cookie": "sid=…", "visible": true },
    { "id": "deepseek-7890ab", "vendor": "deepseek", "label": "",       "key": "", "builtin": 0, "visible": true }
  ]
}
```

- **自配实例**：`key`（glm / minimax / deepseek）或 `cookie`（mimo）明文存本机 config.json，不进 git、不分发
- **内置实例**：`builtin: N` 引用编译进二进制的该品牌第 N 个密文 key（开发者分发场景），`key` 字段恒为空、永不落盘
- 日常管理走托盘「设置…」对话框（增删 / 改名 / 换 key / 显隐 / 排序），一般无需手编文件；至少保留一个实例且至少一个可见

## 打包分发

```bash
pnpm build:tauri
```

- **Windows**：`src-tauri/target/release/aistatus.exe`（免安装直跑）+ `bundle/nsis/*.exe`（NSIS）+ `bundle/msi/*.msi`；构建脚本无条件强制 msvc，产物自包含、无需伴随 DLL
- **macOS**：`.app` + `.dmg`（`src-tauri/target/release/bundle/`）

> ⚠ 未签名的 macOS 应用首次打开需右键 → 打开；未签名的 Windows exe 可能被杀软误报。

「内置实例」分发（收件人免配 key 且看不到明文）的开发者流程见 [AGENTS.md](./AGENTS.md)「API Key 与内置实例」。

## 技术栈

- **Rust** + **Tauri 2**（后端 / 原生集成），tokio + reqwest 按实例并发拉取
- **Svelte 5**（runes）+ **TypeScript** + **Vite 6**
- **window-vibrancy**（原生毛玻璃）
