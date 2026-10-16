# aistatus — AI API 余额监控

桌面常驻悬浮框，监控 GLM 智谱 / MiniMax / 小米 MiMo / DeepSeek 四家 API 余额与配额。
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
- **后端**：Rust + Tauri 2，`tokio` + `reqwest` 并发拉取四家 API
- **存储**：用户数据目录下 `config.json`（主题、悬浮条位置、看板显示的 Coding Plan、MiMo Cookie）+ `balance.json`（余额数据模板）
- **API Key**：GLM / MiniMax / DeepSeek 三家 key **编译期加密硬编码**进二进制（`scripts/encrypt-keys.mjs` 读本机 `src-tauri/keys.local.json` 明文 → XOR 加密 → `src-tauri/secrets.enc` 密文提交 git → `build.rs` 编译期注入字节数组 → `secrets.rs` 运行期解密）。明文 key 不出现在源码 / 二进制字符串表（`strings` 扫不到），挡住静态逆向；用户无需也无法配置 key。`keys.local.json` 是明文源，已 `.gitignore`，绝不提交。**`deepseek` 是可选字段**：没放 key 时 `secrets.enc` 不含 `encDeepseek`，编译出的程序不显示 DeepSeek 卡片。
- **MiMo Cookie（例外，不走编译期加密）**：小米 MiMo 的用量接口（`platform.xiaomimimo.com/api/v1/tokenPlan/usage`，逆向自控制台、非官方公开 API）**不接受 API Key，只认浏览器登录 Cookie**，因此由用户**运行期配置**：设置对话框粘贴（支持纯值 / `Cookie:` 前缀 / cURL 整段三种形态，`normalize_cookie` 清洗）→ 明文存 `config.json` 的 `mimo.cookie` → 保存即触发刷新。每次刷新现读 config，无需缓存失效。Cookie 会过期，失效时卡片显示提示，需重新粘贴。明文存储是经确认的决策：config.json 只落用户本机数据目录、不进 git，与编译期加密（防 key 进分发包）的威胁模型不同。
- **DeepSeek 余额（余额型服务，非配额型）**：官方公开接口 `GET api.deepseek.com/user/balance`（Bearer key），返回 `is_available` + `balance_infos[]`（`currency` / `total_balance` / `granted_balance` / `topped_up_balance`，**金额是字符串**）。DeepSeek 是纯 prepaid 按量计费，没有时间窗口 / 套餐额度概念，**没有「已用百分比」语义**：主窗口走 `ServiceInfo` 的 balance 分支（大号金额 + 充值 / 赠送明细），悬浮条显示金额而非进度条（金额按余额阈值着色：红 <¥10 / 黄 <¥50）。key 与 GLM/MiniMax 同走编译期加密，但属**可选服务**（三态 `DeepseekFetch`）：未内置 key 不出卡片；401 显示「API Key 无效」占位、402/503 显示「余额不足或服务繁忙」占位（对齐 MiMo Cookie 失效的占位思路，比 N/A 可诊断）。多币种时优先取 CNY 条目。
- **外观**：`window-vibrancy` 原生毛玻璃（Win acrylic/mica、mac vibrancy），失败降级 CSS

## 品牌图标（Lobe Icons）

服务商卡片图标来自 [Lobe Icons](https://github.com/lobehub/lobe-icons)（MIT，1600+ AI 品牌图标，覆盖智谱 / MiniMax / 小米 MiMo / DeepSeek / OpenAI / Claude / Qwen / Kimi 等）。**新增服务商时三步接入**：

1. 到 <https://lobehub.com/icons> 搜品牌，记下 slug（如 `zhipu`、`minimax`、`xiaomimimo`、`deepseek`）
2. `node scripts/fetch-icon.mjs <slug> [保存名]` —— 从 unpkg 下载 640×640 light PNG 到 `src/assets/`（自动校验 PNG 魔数，unpkg 失败换 npmmirror 国内源）
3. `ServiceCard.svelte` 的 `logo()` 加 `startsWith` 分支；`balance.json` 条目的 `icon` 字段保留 emoji 作无图兜底

约定统一用 **light 单图 640×640 PNG**（现有 `glm.png` / `minimax.png` / `mimo.png` / `deepseek.png` 同款，项目早期图标与该库同源）。手动下载直链：`https://unpkg.com/@lobehub/icons-static-png@latest/light/<slug>.png`（另有 `dark/`、`*-color.png`、`*-text.png` 变体；SVG 在 `@lobehub/icons-static-svg` 包的 `icons/<slug>.svg`）。图标版权归各品牌方，本工具内为指示性使用。

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
    commands.rs              Tauri commands（get_services / refresh_now / 主题 / 悬浮条 / 看板显隐 / MiMo Cookie …）
    config.rs                config.json 读写（主题、悬浮条位置、MiMo Cookie）
    secrets.rs               内置 API Key 运行期解密（编译期注入密文）
    data.rs / merge.rs / state.rs          数据模型 / 合并 / 状态与拉取调度
    fetcher/                 各家 API 抓取器（glm.rs / minimax.rs / mimo.rs / deepseek.rs）
  resources/balance.json     内嵌首启模板（GLM / MiniMax / MiMo / DeepSeek 占位卡片）
  tauri.conf.json            Tauri 配置（窗口、bundle、identifier）
  Cargo.toml                 Rust 依赖；version 由 sync-version 维护，勿手改
scripts/
  sync-version.mjs           版本号同步（package.json → tauri.conf.json + Cargo.toml）
  build.mjs                  构建 wrapper：自动确保 cargo 在 PATH
  encrypt-keys.mjs           API Key 编译期加密（keys.local.json → secrets.enc）
  fetch-icon.mjs             品牌图标下载（Lobe Icons CDN → src/assets，见「品牌图标」）
```

## 常用命令

```bash
pnpm install          # 装依赖
pnpm dev              # 前端 dev（仅 web，端口 1420）
pnpm tauri dev        # 桌面 dev（热重载）
pnpm build:tauri      # ★ 打包发布（自动处理 cargo PATH，见下）
pnpm sync-version     # 手动同步版本号（build 前会自动跑）
pnpm check            # svelte-check 类型检查
node scripts/encrypt-keys.mjs  # API Key 加密（换 key 时跑，见下「API Key 编译期注入」）
node scripts/fetch-icon.mjs <slug> [保存名]  # 下载品牌图标（见「品牌图标」）
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

### API Key 编译期注入（重要）

GLM / MiniMax / DeepSeek 三家 key **不进 git、不由用户配置**，走编译期加密硬编码（DeepSeek 可选：keys.local.json 不放 `deepseek` 字段则产物不含该 key、不显示 DeepSeek 卡片）。链路：

```
keys.local.json（明文，.gitignore，仅本机）
  └─ scripts/encrypt-keys.mjs ──► secrets.enc（密文，提交 git）
                                   └─ build.rs（编译期）──► OUT_DIR/secrets.rs（字节数组常量）
                                                                   └─ src/secrets.rs（运行期 XOR 解密）
```

**换 key / 换机器 / 全新 clone 时必须做**（否则 `cargo build` / `pnpm tauri dev` 报错「找不到 secrets.enc」）：

1. 创建本机明文源 `src-tauri/keys.local.json`（格式 `{"glm": "...", "minimax": "...", "deepseek": "可选"}`）。此文件**绝不提交**，丢了只能去各家开放平台重新申请。
2. 跑 `node scripts/encrypt-keys.mjs` 生成 `src-tauri/secrets.enc`（密文，提交）。
3. 正常编译。

> ⚠️ `keys.local.json` 是唯一明文源，务必自行安全备份（密码管理器等）。仓库里只有密文，clone 后无法反推明文——这是设计意图（安全性），但意味着明文 key 的保管责任在开发者本地。

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
- 悬浮条显示 GLM / MiniMax 的「5小时限额」与 MiMo 的「Token Plan 套餐」已用%，以及 DeepSeek 的余额金额（不放进度条——余额无分母，金额按余额阈值着色：红 <¥10 / 黄 <¥50；明细进 tooltip）；MiMo 未配置 Cookie / DeepSeek 未内置 key 时对应行隐藏。
- 看板显示哪几个 Coding Plan 由用户在设置对话框勾选（`config.json` 的 `visiblePlans`，存稳定 ID `glm` / `minimax` / `mimo` / `deepseek`，与卡片显示名解耦；**至少一个**，后端 `save_visible_plans` 校验非空，前端把最后一项禁用）。过滤在**展示层**（快照出口：`get_services` / `services-updated` emit）进行，快照始终保留全量——主窗口与悬浮条同步显隐，取消隐藏可立即用缓存回显、随后刷新补数据；后台仍照常拉取全部四家。MiMo 勾选后仍需配置 Cookie 才会出卡片，DeepSeek 勾选后仍需编译期内置 key 才会出卡片。
- 悬浮条高度随可见服务行数自适应（基础 20 + 每行 20 逻辑像素：1 行 40 / 2 行 60 / 3 行 80 / 4 行 100，`state.rs::fit_float_window`，每次刷新后调整；用户未拖过时重锚平台默认角，避免增高压到任务栏。mac 为顶部锚定，`set_size` 后左上角不动，重锚逻辑已按平台分支但未在 mac 实测）。
- 窗口高度自适应卡片数量（`fit_to_content` command + 前端 `ResizeObserver`）。
- 刷新：启动一次 + 每 60s 定时；F5 手动刷新；编辑 `balance.json` 也会触发刷新。
- MiMo 卡片是可选服务（三态）：未配置 Cookie 不显示卡片；偶发拉取失败显示 N/A；Cookie 失效显示明确提示条。MiMo 的 `QuotaItem.used/total` 是**原始额度**（非 0–100 百分比），API 的 `percent` 是 0–1 小数，展示百分比需自行换算（见 `to_quota_item` 与悬浮条 `mimoPct`）。周期重置倒计时来自 `tokenPlan/detail` 的 `currentPeriodEnd`（北京时间字符串），换算成「N天后重置」拼进 Token Plan 套餐条的 detail；该请求失败只少文案，不影响主数据。
- DeepSeek 卡片是可选服务（三态，见 `DeepseekFetch`）：未内置 key 不显示卡片；偶发拉取失败显示 N/A；401 key 无效 / 402、503 余额不足或繁忙显示明确占位提示（占位时金额显示 ¥0.00，原因在 detail）。余额型走 `ServiceInfo` 的 balance 分支（`BalanceView` 大号金额 + detail 充值 / 赠送拆分）；`is_available=false` 时 detail 附「余额不足，请及时充值」。
