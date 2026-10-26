# aistatus — AI API 余额监控

桌面常驻悬浮框，监控 GLM 智谱 / MiniMax / 小米 MiMo / DeepSeek 的 API 余额与配额。
**Coding Plan 以「实例」为粒度**：同品牌可配多份（如 2 个 GLM coding plan、2 个 DeepSeek key），
实例在设置对话框中增删 / 改名 / 换 key / 显隐。
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
- **每次改动都要维护版本号**：升 `package.json` 的 `version` 并随同一提交提交（升档约定与配套动作见「版本管理」）。

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
- **后端**：Rust + Tauri 2，`tokio` + `reqwest` 并发拉取全部 plan 实例
- **存储**：用户数据目录下 `config.json`（主题、悬浮条位置、**plans 实例列表**——含各实例的 key / MiMo Cookie / 显隐，见下）。旧版 `balance.json` 已退役：快照直接由 plans 配置 + 拉取结果生成（`snapshot.rs`），不再有模板文件与按名称合并。
- **Coding Plan 实例（核心模型）**：`config.json` 的 `plans` 数组，每项 `{ id, vendor, label, key, cookie, visible, builtin? }`——`id` 为 `{vendor}-{6位hex}` 稳定实例 ID（后端 `normalize_plans` 生成，卡片 / 悬浮条 / 显隐都以它标识，同品牌多实例各占一卡一行）；`vendor` ∈ `glm / minimax / mimo / deepseek`；glm/minimax/deepseek 用 `key` 字段、mimo 用 `cookie` 字段；`label` 用户备注名（区分同品牌实例）。**两类实例**：①「内置实例」（分发场景）——`builtin: N` 引用编译进二进制的该品牌第 N 个 key，`key` 字段**恒为空、永不落盘**，运行期拉取时才从密文解密（`PlanConfig::credential()`）；②「自配实例」——key 明文存 config.json（与 Cookie 同威胁模型：只落本机数据目录、不进 git），每轮刷新现读文件，增删改即时生效。快照出口（`get_services` / `services-updated`）按每实例 `visible` 过滤。`load_plans` 每轮做「内置 key 明文自清理」：发现与内置 key 相同的明文实例自动转 builtin 引用并落盘清除（旧格式升级 / 重复粘贴收敛）。
- **编译期加密（内置实例的凭据源）**：`scripts/encrypt-keys.mjs` + `secrets.enc` + `build.rs` + `secrets.rs` 链路。keys.local.json 每个品牌字段支持**数组**（同品牌多 key，如 `"glm": [主key, 备用key]`，兼容单字符串），按序加密写入 secrets.enc 的密文数组；`secrets.rs` 暴露 `glm_keys() / minimax_keys() / deepseek_keys() -> Vec<String>`，序号即 `builtin` 取值。config.json 尚无 `plans` 字段时（旧版升级 / 全新环境首启）按内置 key 列表播种实例（`config.rs::migrate_seed_plans`）。`build.rs` 对 secrets.enc 缺失降级为警告 + 空列表（无内置 key 也能构建，实例全部走运行期配置）。
- **MiMo Cookie**：小米 MiMo 的用量接口（`platform.xiaomimimo.com/api/v1/tokenPlan/usage`，逆向自控制台、非官方公开 API）**不接受 API Key，只认浏览器登录 Cookie**：设置对话框粘贴（支持纯值 / `Cookie:` 前缀 / cURL 整段三种形态，`normalize_cookie` 清洗）→ 存 plans 中 `vendor=="mimo"` 实例的 `cookie` 字段（`save_mimo_cookie` 命令签名不变，内部改走 plans）→ 保存即触发刷新。Cookie 过期显示明确提示条，需重新粘贴。
- **DeepSeek 余额（余额型服务，非配额型）**：官方公开接口 `GET api.deepseek.com/user/balance`（Bearer key），返回 `is_available` + `balance_infos[]`（`currency` / `total_balance` / `granted_balance` / `topped_up_balance`，**金额是字符串**）。DeepSeek 是纯 prepaid 按量计费，没有时间窗口 / 套餐额度概念，**没有「已用百分比」语义**：主窗口走 `ServiceInfo` 的 balance 分支（大号金额 + 充值 / 赠送明细），悬浮条显示金额而非进度条（金额按余额阈值着色并附状态标签：安全 ≥¥50 绿 / 预警 ¥10–<¥50 黄 / 危险 <¥10 红；错误占位不标状态）。每个 DeepSeek 实例一张余额卡（可配多个 key）。多币种时优先取 CNY 条目。
- **fetcher 三态（`PlanFetch`，四家统一）**：空凭据 → `Skipped`（不出卡片）；网络 / HTTP / 解析失败 → `Failed`（卡片 loading 显示 N/A）；成功或业务错误 → `Ok(ServiceInfo)`。**认证失败与无数据给明确占位提示而非 N/A**——key 为用户粘贴，必须可诊断：GLM 的认证失败走 **HTTP 200 + 业务码**（`code=401/403`，不是 HTTP 状态码！），按业务码映射「API Key 无效 · 请在设置中检查」；账号无 plan 数据（HTTP 200 但 limits 为空 / 解析出 0 条）显示「未查询到 Coding Plan 用量 · 请确认该账号已开通」；MiniMax / DeepSeek 走 HTTP 401/403（DeepSeek 402/503 显示「余额不足或服务繁忙」）；MiMo Cookie 失效显示「请在设置中重新粘贴」。**GLM 配额解析同时接受 `TOKENS_LIMIT` 与 `CREDIT_LIMIT`**——新版积分制 Coding Plan（如 lite 档）返回 `CREDIT_LIMIT`，字段语义与 token 版一致（`usage`=总量、`currentValue`=已用、`unit/number` 标识窗口：3/5=5小时、6/1=周），仅单位显示为「积分」；`TIME_LIMIT`（工具调用次数）条目仅在积分制套餐（limits 含 CREDIT_LIMIT）保留，token 计量的老套餐不产出该条目（卡片隐藏此行）；两条分支（绝对值数量条 / 纯百分比条）的 detail 统一只放窗口信息——重置倒计时「N小时后重置」，用量不重复进 detail（行右侧已有「X / Y 单位」）；无 `nextResetTime` 且用量为 0（GLM 5 小时窗口未激活时接口不下发该字段）显示「窗口未激活 · 首次调用后开始计时」；百分比条行右侧值显示「N%」（不显示「X/100 %」比值形式）；出现新 `type` 时会静默跳过（解析出 0 条走上述占位），接入新类型改 `fetcher/glm.rs` 的 limit_type 分支。
- **外观**：`window-vibrancy` 原生毛玻璃（Win acrylic/mica、mac vibrancy），失败降级 CSS

## 品牌图标（Lobe Icons）

服务商卡片图标来自 [Lobe Icons](https://github.com/lobehub/lobe-icons)（MIT，1600+ AI 品牌图标，覆盖智谱 / MiniMax / 小米 MiMo / DeepSeek / OpenAI / Claude / Qwen / Kimi 等）。**新增服务商时三步接入**：

1. 到 <https://lobehub.com/icons> 搜品牌，记下 slug（如 `zhipu`、`minimax`、`xiaomimimo`、`deepseek`）
2. `node scripts/fetch-icon.mjs <slug> [保存名]` —— 从 unpkg 下载 640×640 light PNG 到 `src/assets/`（自动校验 PNG 魔数，unpkg 失败换 npmmirror 国内源）。**下载后必须降采样到 128×128 再入库**：卡片显示尺寸仅 24px，128 已覆盖 4×+ DPI，640 原图每张解码后占 ~1.6MB 渲染器内存（脚本结尾会打印降采样命令，PIL / sips 均可）
3. 前端按 `vendor` 稳定 ID 接入三处：`ServiceCard.svelte` 的 `logo()` switch 加分支、`barColors.ts` 的 `barAccent()` 加色相分支、`FloatWidget.svelte` 的 `brandShort()` 加短名；fetcher 产出的 `ServiceInfo.icon` emoji 作无图兜底

约定统一用 **light 单图、入库尺寸 128×128 PNG**（源图取 Lobe Icons 的 640×640 light 变体，下载后降采样；现有 `glm.png` / `minimax.png` / `mimo.png` / `deepseek.png` 同款）。手动下载直链：`https://unpkg.com/@lobehub/icons-static-png@latest/light/<slug>.png`（另有 `dark/`、`*-color.png`、`*-text.png` 变体；SVG 在 `@lobehub/icons-static-svg` 包的 `icons/<slug>.svg`）。图标版权归各品牌方，本工具内为指示性使用。

## 目录结构

```
src/                         前端
  App.svelte                 主壳：标题栏 + 卡片列表 + 页脚 + 右键菜单
  components/                ServiceCard / ConfigDialog / TitleBar / BalanceView / QuotaRow / FloatWidget
  barColors.ts               进度条配色（按 vendor + 周期分色，悬浮条与看板共用）
  balanceStatus.ts           DeepSeek 余额阈值分级（安全 / 预警 / 危险，看板与悬浮条共用）
  stores/services.svelte.ts  余额数据 store（services-updated 事件订阅）
  stores/theme.svelte.ts     主题偏好 store（system/dark/light，跨窗口同步）
  api.ts                     封装 Tauri IPC（commands + events）
  types.ts                   共享类型（ServiceInfo 含 id/vendor/label；PlanConfig 实例配置）
src-tauri/
  src/
    lib.rs                   入口：窗口 / 托盘 / 定位 / 60s 定时刷新
    backdrop.rs              原生毛玻璃
    commands.rs              Tauri commands（get_services / get+save_plans / 主题 / 悬浮条 / MiMo Cookie / 打开配置文件 …）
    config.rs                config.json 读写（主题、悬浮条位置、plans 实例列表 + 播种迁移 + 规范化）
    snapshot.rs              按 plans 并发拉取 + 组装快照 + 出口可见性过滤（fetch_plan / build_snapshot / filter_visible）
    secrets.rs               内置 API Key 解密（仅作升级迁移播种源）
    data.rs / state.rs       数据模型（ServiceInfo / PlanFetch）/ 状态与拉取调度
    fetcher/                 各家 API 抓取器（glm.rs / minimax.rs / mimo.rs / deepseek.rs，统一返回三态 PlanFetch）
  tauri.conf.json            Tauri 配置（窗口、bundle、identifier）
  Cargo.toml                 Rust 依赖；version 由 sync-version 维护，勿手改
scripts/
  sync-version.mjs           版本号同步（package.json → tauri.conf.json + Cargo.toml）
  build.mjs                  构建 wrapper：自动确保 cargo 在 PATH
  encrypt-keys.mjs           API Key 编译期加密（可选：生成播种源 secrets.enc，见「API Key 与播种」）
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
node scripts/encrypt-keys.mjs  # 可选：生成播种源密文（见下「API Key 与播种」）
node scripts/fetch-icon.mjs <slug> [保存名]  # 下载品牌图标（见「品牌图标」）
```

## 构建与打包

`pnpm build:tauri` 产物：

- `src-tauri/target/release/aistatus.exe`（免安装直跑）
- `src-tauri/target/release/bundle/nsis/AI余额监控_<ver>_x64-setup.exe`（NSIS 安装包）
- `src-tauri/target/release/bundle/msi/AI余额监控_<ver>_x64_zh-CN.msi`（MSI）

### 构建环境前提（Windows，重要）

1. **必须用 MSVC 工具链** `stable-x86_64-pc-windows-msvc`。gnu 产物有两个坑：
   - `cdylib` 导出符号数会撞 GNU ld 序号上限（>65535），链接失败；
   - WebView2 加载器在 gnu 下是**动态链接**（msvc 下静态链接进 exe）——gnu 版 exe 单独分发
     到其他机器必报「找不到 WebView2Loader.dll」（2026-10-22 实际翻车；装 WebView2 运行时
     无效，缺的不是运行时），NSIS 安装包也只装 exe 不带这个 DLL。分发前可用
     `objdump -p aistatus.exe | grep -i webview` 自检：msvc 产物导入表应无 `WebView2Loader.dll`。
2. **需要 Visual Studio Build Tools**（MSVC linker `link.exe`）；rustc 通过 vswhere/注册表自动发现，无需手动 vcvars。
3. **cargo 必须在 PATH**。本机坑：`~/.cargo/bin` 下 rustup 代理 stub 缺失，`cargo` 找不到。
   - `pnpm build:tauri`（`scripts/build.mjs`）Windows **无条件强制 msvc**：把
     `~/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin` 置于 PATH 最前并设
     `RUSTUP_TOOLCHAIN`，即使 `~/.cargo/bin/cargo.exe`（rustup 代理，会解析到默认工具链 gnu）
     存在也不走 gnu；msvc toolchain 缺失时告警回退。
   - 根治（重建 stub，需自行在 PowerShell 运行）：
     ```powershell
     Invoke-WebRequest https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
     .\rustup-init.exe -y --default-toolchain stable-x86_64-pc-windows-msvc
     ```
4. **手工跑 cargo 命令时（`cargo check` / `cargo test`）要显式指定 msvc，且把它所在目录加进 PATH**：
   ```bash
   export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
   cargo test
   ```
   两个独立坑，症状不同：
   - **默认工具链是 gnu**：`~/.cargo/bin/cargo.exe`（rustup 代理）按 default 解析，而本机
     default 指向 `stable-x86_64-pc-windows-gnu`，直接 `cargo test` 会以第 1 条的
     `error: export ordinal too large: 117286` 链接失败；用上面的 PATH 才会解析到 msvc。
   - **`rustc` 不在 PATH**：`embed-resource` 的构建脚本会 `Command::new("rustc")` 取版本，
     用绝对路径调 cargo.exe 时 rustc 找不到，构建脚本 panic
     `couldn't get rustc version: CouldNotExecuteCommand(... program not found)`。
     只要 `build.rs`（或依赖的构建脚本）需要重新编译就会暴露，改过 build.rs 后尤甚。

> ⚠️ 「用 `&&` 串联 + `| tail` 看输出」时，cargo 失败也会被 `tail` 吞掉退出码，看起来像成功。
> 检查请用 `cargo check; echo $?`，或 `&& echo "exit=$?"` 之前不要接管道。

### API Key 与内置实例（分发场景，重要）

Key 有两类来源，对应两种使用方式：

**① 内置实例（开发者分发 exe 给他人时用）**——key 只以密文存在于二进制，**永不落盘**：

```
keys.local.json（明文，.gitignore，仅本机；每品牌支持数组 = 同品牌多 key）
  └─ scripts/encrypt-keys.mjs ──► secrets.enc（密文数组，提交 git）
                                   └─ build.rs（编译期）──► OUT_DIR/secrets.rs（字节数组常量）
                                                                   └─ src/secrets.rs（运行期解密）
```

config.json 里内置实例只存 `{builtin: 序号}` 引用 + 备注 / 显隐，**不含 key 明文**——收件人打开 config.json 也看不到 key；拉取时才从二进制密文解密、仅在内存短暂存在。安全边界与旧版一致：挡 strings / 静态扫描提取；不防有决心的逆向或运行期内存 dump（客户端密钥的固有边界）。

**② 自配实例（自己用 / 收件人加自己的 key）**——在设置对话框添加，key 明文存 config.json 的 `plans` 数组，不进 git、不分发，威胁模型与 MiMo Cookie 相同。换 Key 会把内置实例「断开内置」转为自配实例。

**开发者工作流（换 key / 加 key / 出分发版）**：

1. 编辑 `src-tauri/keys.local.json`：`{"glm": ["主key", "备用key"], "minimax": ["..."], "deepseek": ["..."]}`（deepseek 可选；数组顺序 = builtin 序号，**改顺序会改变已有内置实例的对应关系**，插在末尾最安全）。**绝不提交**，自行安全备份。
2. `node scripts/encrypt-keys.mjs` 重新生成 `secrets.enc`（提交 git）。
3. 重新编译。开发者本机 config.json 里已有的同值明文实例会在下次启动时**自动转为 builtin 引用**并清除明文（自清理，无需手动改）。
4. `pnpm build:tauri` 出分发包：收件人首启自动按内置 key 列表播种实例（继承旧 `visiblePlans` 显隐与 `mimo.cookie`），无需配置任何 key。

**全新 clone / keyless 构建**：`secrets.enc` 缺失时 `build.rs` 只警告不报错（空列表），构建出的程序无内置实例，全部在设置对话框运行期配置。

> ⚠️ `keys.local.json` 是明文源，务必自行安全备份（密码管理器等）。运行期配置的 key 存在用户本机 config.json，同样建议用户自行备份该文件。

### 打包注意

- **WiX 语系必须是 `zh-CN`**（`tauri.conf.json` → `bundle.windows.wix.language`）。中文 `productName`（AI余额监控）在默认 en-US / 代码页 1252 下会触发 `LGHT0311`。
- **bundle identifier 不要以 `.app` 结尾**（与 macOS 应用包扩展名冲突）。当前：`com.aistatus.desktop`。
- **pnpm 11 需在 `pnpm-workspace.yaml` 授权 esbuild**（`onlyBuiltDependencies`），否则 `pnpm install` 退出码 1。`package.json` 不再放 `pnpm` 字段（pnpm 11 已不读取，会打 WARN）。

## 版本管理

**单一真相源 = `package.json` 的 `version`**，勿单独改 `tauri.conf.json` 或 `Cargo.toml`。

**每次改动都要维护版本号（硬性规则）**：任何代码 / 行为改动——新功能、缺陷修复、UI / 文案调整、依赖或构建变更——都必须在**同一提交**里升 `package.json` 的 `version`，保证每个可构建产物都有唯一版本可区分。升档约定：新功能 / 行为变化 +0.1.0（如 0.4.0 → 0.5.0），缺陷修复 / 文案微调 +0.0.1（如 0.4.0 → 0.4.1）；纯文档（本文件 / README / todo.md）与 CI 调整不升版本。升完跑 `pnpm sync-version` 同步 `tauri.conf.json` + `Cargo.toml`（build 前也会自动跑），`Cargo.lock` 的版本行随之更新、一并提交。

发新版本流程：

1. 改 `package.json` 的 `version`（如 `0.2.0`）
2. `pnpm build:tauri` —— `beforeBuildCommand` 自动：
   - 跑 `sync-version`，把版本同步到 `tauri.conf.json` + `Cargo.toml`（产物文件名随之带新版本）
   - Vite 把 `__APP_VERSION__` / `__BUILD_TIME__` / `__GIT_HASH__` 注入前端（见 `vite.config.js` 的 `define`）
3. 页脚显示 `v0.2.0 · 06-30 10:22 · fd420fc`，每次构建的「时间 + git hash」不同，可一眼区分新旧产物。

## 运行时行为

- 窗口默认隐藏，托盘菜单「打开主界面」唤起；mac 关闭按钮=隐藏（常驻托盘），仅「退出」结束进程。
- 定位：Windows 锚主屏右下角（避任务栏，逻辑像素×缩放）；mac 跟随托盘图标。
- **多实例渲染**：每条 plan 实例一张卡片 / 悬浮条一行，按 plans 配置顺序排列。卡片标题 = 品牌展示名（GLM 带「· {LEVEL}」套餐后缀）+ 用户 `label`（弱化色追加）；悬浮条行标签 = **品牌短名 + label**（如「GLM 主账号」，无 label 就是品牌短名）——品牌必须可见，备注只做区分；品牌列**统一固定 76px**（各行进度条起点对齐；`min-width: 0` 必须显式声明，否则 nowrap 长文字把 grid 项撑出列外压到进度条，超长备注截断省略）。前端图标 / 配色 / 分组按 `ServiceInfo.vendor` 稳定 ID 匹配，不依赖显示名；卡片列表 each key 用 `svc.id`。
- 悬浮条数据驱动渲染可见服务实例：quota 型若 items 含「5小时限额」+「周限额」两条则同一行上下双 4px 进度条（GLM / MiniMax；上=5小时、下=本周，数值在 tooltip），否则单条 + 右侧百分比（MiMo 的「Token Plan 套餐」）；balance 型（DeepSeek）不放进度条——余额无分母，右对齐金额按阈值着色（红 <¥10 / 黄 <¥50；明细进 tooltip）。进度条填充为同色相浅→深渐变，色相按 vendor + 周期固定（GLM 冷色组 5小时电光蓝 `#3B82F6` / 周翡翠绿 `#34D399`，MiniMax 暖色组亮洋红 `#D946EF` / 珊瑚橙 `#FB923C`，MiMo 橙 `#FF6900`；**同品牌多实例共用色相，靠行标签区分**）；**仅 GLM 的 5 小时条为固定色带**：淡蓝 `#a9cdff` → 品牌蓝 `#3b82f6` → 紫罗兰 `#7c3aed` → 玫红 `#be123c` → 深红 `#B91C1C` 的五段渐变（`barColors.ts::glmFiveHSpectrum`；中间锚点既要避免蓝→红直插的发灰绛紫，也要避免取比品牌蓝更亮的淡紫——那会让后半段出现「发白变浅」的色带，各锚点相对明度须单调下降），配合 `gradientSize` 按填充宽度从左往右揭示——低用量只见淡蓝端、越高露出越多红端、满格完整呈现，丝滑过渡无阈值跳变；其余条目（周限额、MiniMax / MiMo）色相恒定、**不做阈值变色**：填充为同色相「浅→深」渐变，随填充变宽露出更深的颜色（用量语义由深浅表达，全程无跳变）。
- 主看板配额行（`QuotaRow`）的进度条与悬浮条共用 `src/barColors.ts`：同样按 vendor + 周期分色、同色相浅→深渐变，仅 GLM 5 小时条为淡蓝→深红固定色带，其余条目不做阈值变色；`periodOfLabel` 按 label 是否含「周」判定 `week`，MiMo「Token Plan 套餐」等其余条目按单条处理。卡片标题 / 图标仍用各实例品牌色（`service.color`），与数据条颜色解耦。
- **实例显隐、增删与排序**在设置对话框「Coding Plan 管理」区完成（`config.json` 的 `plans`，每实例 `visible`；**至少保留一个实例、至少一个可见**，后端 `save_plans` 校验、前端禁用最后一项的删除 / 取消勾选）。**显示顺序 = plans 数组顺序**（主窗口卡片与悬浮条一致），列表行内 ↑↓ 按钮调序、整表乐观保存。过滤在**展示层**（快照出口：`get_services` / `services-updated` emit）进行，快照始终保留全量——主窗口与悬浮条同步显隐，取消隐藏可立即用缓存回显、随后刷新补数据；隐藏实例后台仍照常拉取。MiMo 实例需 Cookie 才出卡片，其余实例需 key（内置实例凭据走编译期密文）。右键菜单「编辑配置文件」用系统默认编辑器打开 config.json。
- 悬浮条宽 220（品牌列 76 + 进度条 ~118；改宽需同步 `state.rs` 的 `FLOAT_WIDTH` 与 `tauri.conf.json` 的 float 窗口宽度两处）；高度随可见服务行数自适应（基础 20 + 每行 20 逻辑像素，多实例按实际行数增长，`state.rs::fit_float_window`，每次刷新后调整；宽度不符时一并修正，保存过位置的窗口加宽后左移补偿防止贴右缘出屏；用户未拖过时重锚平台默认角，避免增高压到任务栏。mac 为顶部锚定，`set_size` 后左上角不动，重锚逻辑已按平台分支但未在 mac 实测）。
- 窗口高度自适应卡片数量（`fit_to_content` command + 前端 `ResizeObserver`）；最大化 / 全屏时跳过自适应（`set_size` 会把窗口拉出最大化态，表现为最大化无效），还原后恢复。
- 刷新：启动一次 + 每 60s 定时；F5 手动刷新。plans 每轮现读 config.json，实例增删 / 换 key / 改 Cookie 无需重启。
- MiMo 卡片是可选服务（三态 `PlanFetch`）：未配置 Cookie 不显示卡片；偶发拉取失败显示 N/A；Cookie 失效显示明确提示条。MiMo 的 `QuotaItem.used/total` 是**原始额度**（非 0–100 百分比），API 的 `percent` 是 0–1 小数，展示百分比需自行换算（见 `to_quota_item` 与悬浮条 `usedPct`）。周期重置倒计时来自 `tokenPlan/detail` 的 `currentPeriodEnd`（北京时间字符串），换算成「N天后重置」拼进 Token Plan 套餐条的 detail；该请求失败只少文案，不影响主数据。
- DeepSeek 每实例一张余额卡（可配多个 key）：空 key 不显示卡片（`Skipped`）；偶发拉取失败显示 N/A；401 key 无效 / 402、503 余额不足或繁忙显示明确占位提示（占位时金额显示 ¥0.00，原因在 detail）。余额型走 `ServiceInfo` 的 balance 分支（`BalanceView` 大号金额 + detail 充值 / 赠送拆分）；金额旁显示状态标签（安全 ≥¥50 绿 / 预警 ¥10–<¥50 黄 / 危险 <¥10 红），`placeholder=true` 的错误占位不显示状态；`is_available=false` 时 detail 附「余额不足，请及时充值」。
