# aistatus — AI API 余额监控

桌面常驻悬浮框，监控 DeepSeek / GLM 智谱 / MiniMax 三家 API 余额与配额。
Tauri 2 + Svelte 5 + TypeScript + Vite 跨平台桌面实现。

> 本文件是项目开发规范的唯一来源。`CLAUDE.md` 仅负责引用本文件，
> 规则更新统一修改 `AGENTS.md`，不要在多个说明文件中重复维护。

## 开发规则

- **每次改动都要考虑多平台支持**。本项目同时面向 Windows 与 macOS（Linux 次要），任何代码 / 脚本 / 配置改动都必须在所有目标平台上行为正确：
  - 路径用 `path.join`，不硬编码分隔符或绝对路径。
  - 脚本命令在 `cmd.exe`（Win）与 `sh`（mac/Linux）都要能跑（`&&` 两者都支持；避免 bash-only 语法）。
  - 平台差异用 `#[cfg(target_os = ...)]`（Rust）或 `process.platform`（Node）显式分支，不假设单一平台。
  - 涉及平台相关行为（毛玻璃、窗口定位、托盘、菜单栏）的改动，在本文档标注各平台差异。
  - 尽量在对应平台验证；本机无法验证的平台（如本机为 Windows 时改了 mac 代码），要在提交说明里标注「未在某平台实测」并附验证建议。

## TODO 清单（todo.md）

跨平台开发中，常有些事项**无法在本机立即确认**（如本机为 Windows 时改了 macOS 代码），或改动产生的后续待办。这类事项记录在仓库根目录的 `todo.md`，避免遗忘。

**格式约定**：
- 用 Markdown 任务列表：`- [ ]` 未完成、`- [x]` 已完成（完成后改标记并注明日期）。
- 分组：「🔬 无法在本机确认」（需对应平台验证）、「🧹 其他待办」（非阻塞）。
- 每项写清三段：**背景**（为什么产生）、**验证/操作方法**（怎么做）、**完成判据**（怎样算完成）。

**使用时机**：
- 改动涉及当前环境无法验证的平台 / 配置 → 记一条，标未完成。
- 发现需要后续处理的非阻塞问题（根治方案、清理、提交）→ 记一条。

**维护**：完成一项就把 `[ ]` 改 `[x]` 并补日期；若全部完成，清空或归档该文件，不要长期保留过时条目。

## 技术栈

- **前端**：Svelte 5（runes）+ TypeScript + Vite 6，目标 `es2021`
- **后端**：Rust + Tauri 2，`tokio` + `reqwest` 并发拉取三家 API
- **存储**：用户数据目录下 `config.json`（API Key）+ `balance.json`（余额；前端监听文件变更防抖刷新）
- **外观**：`window-vibrancy` 原生毛玻璃（Win acrylic/mica、mac vibrancy），失败降级 CSS

## 目录结构

```
src/                         前端
  App.svelte                 主壳：标题栏 + 卡片列表 + 页脚 + 右键菜单
  components/                ServiceCard / ConfigDialog / TitleBar / BalanceView / QuotaRow
  stores/services.svelte.ts  余额数据 store（services-updated 事件 + 文件变更）
  api.ts                     封装 Tauri IPC（commands + events）
  types.ts                   共享类型
src-tauri/
  src/
    lib.rs                   入口：窗口 / 托盘 / 定位 / 60s 定时刷新
    backdrop.rs              原生毛玻璃
    commands.rs              Tauri commands（get_services / save_keys / …）
    config.rs                API Key 读写
    data.rs / fetcher.rs / merge.rs / state.rs   拉取 / 合并 / 状态
  tauri.conf.json            Tauri 配置（窗口、bundle、identifier）
  Cargo.toml                 Rust 依赖；version 由 sync-version 维护，勿手改
scripts/
  sync-version.mjs           版本号同步（package.json → tauri.conf.json + Cargo.toml）
  build.mjs                  构建 wrapper：自动确保 cargo 在 PATH
```

## 常用命令

```bash
pnpm install          # 装依赖
pnpm dev              # 前端 dev（仅 web，端口 1420）
pnpm tauri dev        # 桌面 dev（热重载）
pnpm build:tauri      # ★ 打包发布（自动处理 cargo PATH，见下）
pnpm sync-version     # 手动同步版本号（build 前会自动跑）
pnpm check            # svelte-check 类型检查
```

## 构建与打包

`pnpm build:tauri` 产物：

- `src-tauri/target/release/aistatus.exe`（免安装直跑）
- `src-tauri/target/release/bundle/nsis/AI余额监控_<ver>_x64-setup.exe`（NSIS 安装包）
- `src-tauri/target/release/bundle/msi/AI余额监控_<ver>_x64_zh-CN.msi`（MSI）

### 构建环境前提（Windows，重要）

1. **必须用 MSVC 工具链** `stable-x86_64-pc-windows-msvc`。
   gnu target 的 `cdylib` 导出符号数会撞 GNU ld 序号上限（>65535），链接失败。
2. **需要 Visual Studio Build Tools**（MSVC linker `link.exe`）；rustc 通过 vswhere/注册表自动发现，无需手动 vcvars。
3. **cargo 必须在 PATH**。本机坑：`~/.cargo/bin` 下 rustup 代理 stub 缺失，`cargo` 找不到。
   - `pnpm build:tauri`（`scripts/build.mjs`）自动兜底：若 `~/.cargo/bin/cargo.exe` 不存在，回退到 `~/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin`。
   - 根治（重建 stub，需自行在 PowerShell 运行）：
     ```powershell
     Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
     .\rustup-init.exe -y --default-toolchain stable-x86_64-pc-windows-msvc
     ```

### 打包注意

- **WiX 语系必须是 `zh-CN`**（`tauri.conf.json` → `bundle.windows.wix.language`）。中文 `productName`（AI余额监控）在默认 en-US / 代码页 1252 下会触发 `LGHT0311`。
- **bundle identifier 不要以 `.app` 结尾**（与 macOS 应用包扩展名冲突）。当前：`com.aistatus.desktop`。
- **pnpm 11 需在 `pnpm-workspace.yaml` 授权 esbuild**（`onlyBuiltDependencies`），否则 `pnpm install` 退出码 1。`package.json` 不再放 `pnpm` 字段（pnpm 11 已不读取，会打 WARN）。

## 版本管理

**单一真相源 = `package.json` 的 `version`**，勿单独改 `tauri.conf.json` 或 `Cargo.toml`。

发新版本流程：

1. 改 `package.json` 的 `version`（如 `0.2.0`）
2. `pnpm build:tauri` —— `beforeBuildCommand` 自动：
   - 跑 `sync-version`，把版本同步到 `tauri.conf.json` + `Cargo.toml`（产物文件名随之带新版本）
   - Vite 把 `__APP_VERSION__` / `__BUILD_TIME__` / `__GIT_HASH__` 注入前端（见 `vite.config.js` 的 `define`）
3. 页脚显示 `v0.2.0 · 06-30 10:22 · fd420fc`，每次构建的「时间 + git hash」不同，可一眼区分新旧产物。

## 运行时行为

- 窗口默认隐藏，托盘菜单「打开主界面」唤起；mac 关闭按钮=隐藏（常驻托盘），仅「退出」结束进程。
- 定位：Windows 锚主屏右下角（避任务栏，逻辑像素×缩放）；mac 跟随托盘图标。
- 窗口高度自适应卡片数量（`fit_to_content` command + 前端 `ResizeObserver`）。
- 刷新：启动一次 + 每 60s 定时；F5 手动刷新；编辑 `balance.json` 也会触发刷新。
