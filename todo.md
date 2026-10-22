# TODO — 待确认 / 待办事项

> 本次（2026-06-30）改动中，**本机为 Windows、无法在此验证** 的事项，以及相关后续待办。
> 用法与格式约定见 `CLAUDE.md`「TODO 清单（todo.md）」。

---

## 🔬 无法在本机确认（需对应平台验证）

### [x] 1. macOS / Linux：验证内嵌 balance.json 首次启动模板（已随多实例改造失效，2026-10-20）

**背景**：旧实现通过 `include_str!` 将 `resources/balance.json` 编译进程序作首启模板。**2026-10-20 多实例改造后 balance.json 已整体退役**：模板文件、`load_balance_or_default` 与按名称 merge 全部删除，快照直接由 config.json 的 `plans` 实例 + 拉取结果生成，本条不再有可验证对象。

**后续**：首启 / 升级行为改由「播种迁移」承担，mac 端验证并入第 11 条（迁移与多实例实测）。

### [x] 2. macOS 编译验证：`lib.rs` 的条件编译改动（2026-09-01）

**背景**：为消除 Windows 编译告警（unused import / unused variable），给以下两项加了 `#[cfg(target_os = "macos")]`：
- `use tauri::Rect;`（macOS 的 `anchor_near_tray(&Rect)` 与 `rect_opt: Option<tauri::Rect>` 使用）
- `let handle = app.handle().clone();`（macOS 分支 `handle.tray_by_id("main")` 使用）

**当前状态**：Windows 已编译通过（0 warning）。macOS 上两者都在 `#[cfg(target_os = "macos")]` 分支内被实际使用，逻辑确定安全，但本机无 mac 环境无法实测。

**验证方法**（在 mac 上，任选其一）：
```bash
cargo check --manifest-path src-tauri/Cargo.toml
# 或直接出包：
pnpm build:tauri
```
**完成判据**：`cargo check` 无错无 warning；`pnpm build:tauri` 正常产出 `.app`。

**已完成（2026-09-01）**：新增 `.github/workflows/mac-check.yml`（GitHub Actions，macos-latest / Apple Silicon），每次 push 到 main / V2 / develop 自动跑 `cargo check --all-targets` + `cargo test`。首次运行（run #1，commit a9ce39a）全步骤通过：cargo check 86s、cargo test 87s，全部单测在 mac 上通过。`use tauri::Rect` / `let handle` 两项 cfg 改动在真 mac 编译器下确认无告警。此后该项由 CI 持续兜底，无需单独关注；`pnpm build:tauri` 出 `.app` 包仍需真机验证（运行时行为不在 CI 范围，见第 6/7 条）。

### [ ] 3. macOS / Linux：`build.mjs` 的 `findToolchainBin` 兜底实测

**背景**：`scripts/build.mjs` 按 `process.platform` / `process.arch` 选 toolchain：
- Windows → `windows-msvc`（已验证回退成功）
- macOS → `apple-darwin`（自动区分 `aarch64` / `x86_64`）
- Linux → `unknown-linux`

**当前状态**：Windows 端已验证（构建日志 `回退到 ...\stable-x86_64-pc-windows-msvc\bin`）。mac / Linux 未实测。

**验证方法**（在 mac / Linux 上，模拟 stub 缺失）：
```bash
mv ~/.cargo/bin/cargo ~/.cargo/bin/cargo.bak     # 临时让 stub 不可用
pnpm build:tauri                                  # 应见 [build] ... 回退到 ~/.rustup/toolchains/...
mv ~/.cargo/bin/cargo.bak ~/.cargo/bin/cargo      # 还原
```
> 正常环境（`~/.cargo/bin` 有 stub）则直接走 `cargoBin` 分支，无需回退。

**完成判据**：mac / Linux 上 `pnpm build:tauri` 成功产出 `.app` / AppImage。

**已验证（2026-10-19，macOS 正常路径）**：`pnpm build:tauri` 成功产出 `.app` 与 `.dmg`（x64）。本机 `~/.cargo/bin` stub 存在，走正常分支；`findToolchainBin` 回退分支仍未实测（需临时移走 cargo stub）。

### [ ] 6. macOS / Linux：悬浮球（float 窗口）定位、透明度与显隐实测

**背景**：新增了第二个窗口 `label: "float"`（72×72 透明置顶悬浮球），定位逻辑：
- Windows / Linux：复用 `anchor_to_bottom_right`（右下角，避任务栏 48px）—— Windows 本机已验证编译通过，**未实测运行**。
- macOS：复用 `anchor_top_right`（右上角菜单栏下方）—— 本机无 mac 无法实测。
- 启动后由后端 `win.show()` 显示；托盘「悬浮球」`CheckMenuItem` 切换显隐。
- 位置持久化到 `config.json` 的 `floatWindow: {x, y}`（逻辑像素），拖拽结束防抖落盘。

**当前状态**：Windows `cargo check` + `pnpm check` 均通过（0 error 0 warning）。mac / Linux 透明窗口依赖系统合成器，Linux 下若合成器不支持透明可能退化为不透明黑底。

**验证/操作方法**（在 mac / Linux 上）：
```bash
pnpm tauri dev
# 观察：悬浮球是否出现在预期位置、背景是否透明、拖拽后重启位置是否恢复
# 托盘菜单「悬浮球」勾选态切换是否正常 show/hide
```
**完成判据**：
1. 悬浮球在 mac 右上角 / Linux 右下角正确出现，背景透明（露出桌面）。
2. 拖拽后重启 dev，位置恢复到上次落点。
3. 托盘菜单「悬浮球」项切换显隐且勾选态同步。
4. 60s 刷新后数字与颜色随 GLM/MiniMax 最低余量更新。

**已验证（2026-10-19，macOS）**：右上角出现、背景透明、持久化位置可恢复、60s 刷新后金额变化均正常；托盘「悬浮球」勾选切换因无 GUI 自动化未实测。

### [ ] 7. macOS：悬浮条高度自适应（fit_float_window）实测

**背景**：悬浮条按可见服务行数自适应高度：`state.rs::fit_float_window` =「基础 20 + 每行 20 逻辑像素」（2026-10-14 随看板显隐泛化；2026-10-20 多实例改造后按**实例行数**增长，同品牌多实例各占一行，不再按品牌去重）。平台差异：Win/Linux 底部锚定，增高后重锚右下角防压任务栏（**Windows 已实测**：60/80 两态 + 多实例 3 行 80 高均正常）；mac 走 `anchor_top_right` 重锚分支，本机无法验证。
**验证/操作方法**（在 mac 上）：
```bash
pnpm tauri dev
# 配置 MiMo Cookie / 添加多个实例后观察：悬浮条随行数增高（如 5 行 = 120）
# 在设置对话框隐藏 / 删除实例：行数减少、高度回落；增高后窗口顶部应仍贴菜单栏下方
```
**完成判据**：mac 上高度随可见实例行数切换正常、顶部锚定不漂移，单行可见时窗口保持 40 高。

**已验证（2026-10-19，macOS）**：三家可见时高度 80（3×20+20）正确。实测发现并修复 `anchor_top_right` 的 HiDPI 单位 bug：菜单栏高度 25 被当作物理像素，Retina 2x 下算出的 y 仅 14.5 逻辑像素，被系统钳到菜单栏下沿（25），浮窗紧贴菜单栏无间隙；改为按 `scale_factor` 换算后窗口 y=29，恢复 4pt 间隙（已合入 `src-tauri/src/lib.rs`）。动态增减行数的切换未做 GUI 实测。

### [ ] 8. Windows GUI / macOS：Coding Plan 实例管理（plans）实测

**背景**：2026-10-20 多实例改造，`visiblePlans` 品牌级勾选**退役**，改为设置对话框「Coding Plan 管理」：实例列表（显隐勾选 / 备注改名 / 换 Key / 删除）+ 添加表单（GLM / MiniMax / DeepSeek），存 `config.json` 的 `plans`；至少保留一个实例、至少一个可见。后端逻辑已由单测覆盖（normalize / 播种 / 快照过滤），**对话框 GUI 交互未做自动化实测**（Windows 冒烟只验证了展示与数据链路），mac 本机无法验证。
**验证/操作方法**：
```bash
pnpm tauri dev   # 托盘右键 →「设置…」
# 1. 添加实例：选品牌 + 粘贴 key + 备注 → 主窗口与悬浮条各多一行，1–2s 内出数据
# 2. 同品牌加两个实例：备注名正确区分；两实例进度条同色相、靠标签区分
# 3. 换 Key：粘贴错 key 保存 → 卡片显示「API Key 无效 · 请在设置中检查」占位
# 4. 显隐：取消勾选某实例 → 两窗口同步消失；最后一个可见项勾选框禁用
# 5. 删除：剩最后一个实例时删除按钮禁用；删除后快照不再出现该卡片
# 6. 重启应用：实例列表 / 备注 / 显隐全部保持（config.json 的 plans）
# 7. MiMo 行：Cookie 尾部指纹与下方 Cookie 区块联动（保存/清除后同步）
```
**完成判据**：上述 7 项全部符合；增删时主窗口高度、悬浮条高度自适应正常，无残留占位。

### [x] 10. DeepSeek 卡片真 key 验证（需先内置 key，2026-10-15）

**背景**：新增 DeepSeek 余额监控（官方 `GET api.deepseek.com/user/balance` 接口，key 编译期加密注入）。本机 `keys.local.json` 尚无 `deepseek` 字段，卡片处于 NotConfigured（不出卡片），已验证的是：解析/合并单测全绿 + 无 key 时卡片正确隐藏，**真实拉取与展示未验证**。

**验证/操作方法**：
1. 到 [platform.deepseek.com](https://platform.deepseek.com) 创建 API key，加入 `src-tauri/keys.local.json`（`"deepseek": "sk-..."`）
2. `node scripts/encrypt-keys.mjs` 重新生成 `secrets.enc`（会连 GLM/MiniMax 一起重新加密，属正常）
3. `pnpm tauri dev`；若主窗口没出现 DeepSeek 卡片，到设置对话框勾选 DeepSeek（存量 `config.json` 的 `visiblePlans` 不含 `deepseek`，需手动勾选一次；勾选即时生效）

**完成判据**：主窗口出现 DeepSeek 余额卡片（大号金额 + 「充值 ¥x · 赠送 ¥y」明细）；悬浮条底部出现 DeepSeek 金额行（余额 <10 变红）；故意改错 key 重编译后卡片显示「API Key 无效」占位。

**已完成（2026-10-15）**：真实 key 已写入 `keys.local.json` 并重新加密（`secrets.enc` 含三家密文，待随下次提交进 git）。curl 直调接口 HTTP 200，响应字段与解析器逐一吻合（`is_available` / `balance_infos[].currency=CNY` / 字符串金额）；假 key 直调确认 HTTP 401（占位路径触发条件成立，映射逻辑由单测覆盖）。`pnpm tauri dev` 启动后连续多个刷新周期 `[DeepseekFetcher]` 零错误日志（该 fetcher 仅失败路径打日志）。存量 `visiblePlans` 已补 `deepseek`。注：该 key 曾在对话中明文传递，如有顾虑建议在开放平台轮换。

### [ ] 11. macOS / Linux：plans 播种迁移与多实例运行实测（2026-10-20）

**背景**：多实例改造（key 运行期配置 + balance.json 退役 + 旧 `visiblePlans`/`mimo.cookie` 播种迁移）仅在 **Windows 实测**：升级首启自动生成 `plans`（GLM/MiniMax/DeepSeek 可见 + MiMo 保留原隐藏态与 Cookie，`visiblePlans` 字段移除），主窗口与悬浮条多实例渲染、`fit_float_window` 行数自适应均正常。迁移写文件（`config.rs::migrate_seed_plans`）与快照链路是跨平台纯逻辑，mac 风险低，但路径 / 原子写（tmp+rename）/ 时序未在 mac 实测。

**验证/操作方法**（在 mac 上）：
```bash
# 用一份旧版 config.json（含 visiblePlans / mimo.cookie，无 plans 字段）启动新版：
pnpm tauri dev
# 1. 确认 config.json 自动生成 plans 数组、visiblePlans 移除、卡片原样出现
# 2. 设置对话框添加同品牌第二实例，两窗口各多一行
# 3. 右键「编辑配置文件」能打开 config.json
```
**完成判据**：迁移一次性完成且幂等（二次启动不再重写）；卡片 / 悬浮条与 Windows 行为一致。

### [ ] 15. Windows：新版（msvc）exe / 安装包在另一台电脑免 DLL 直跑实测（2026-10-22）

**背景**：此前 `pnpm build:tauri` 实际走的是 rustup 默认工具链 gnu（`~/.cargo/bin` 的 rustup 代理按 default 解析，`build.mjs` 的 msvc 兜底只在 stub 缺失时生效），产出 gnu 版 exe——WebView2 加载器是动态链接，单独分发 exe 必报「找不到 WebView2Loader.dll」，且 NSIS 安装包只装 exe 不带该 DLL（装 WebView2 运行时无效，缺的不是运行时）。`scripts/build.mjs` 已改为 Windows 无条件强制 msvc：msvc 下加载器静态链接进 exe，产物自包含。本机已验证新 exe 导入表无 `WebView2Loader.dll`；「另一台电脑直跑」无法在本机确认。
**验证/操作方法**（在另一台 Windows 电脑上）：只拷新版 `src-tauri/target/release/aistatus.exe`（或重装 `bundle/nsis/AI余额监控_0.4.0_x64-setup.exe`）到不含任何 DLL 的目录直接启动；若目标机缺 WebView2 运行时（部分 Win10 精简版 / Server），用 NSIS 安装包会自动下载。
**完成判据**：不伴随任何 DLL 直跑成功，托盘出现并能打开主界面。

---

## 🧹 其他待办（非阻塞）

### [ ] 13. 内置实例（builtin 引用）模型的 mac / 分发实测（2026-10-20）

**背景**：为维持「exe 分发 + key 不泄露」能力，内置 key 改为**只存二进制密文、不落盘**：config.json 里内置实例仅存 `{builtin: 序号}` 引用（key 字段恒空），运行期拉取时解密；`keys.local.json` 支持数组（同品牌多 key）；`load_plans` 每轮自动把误存的内置 key 明文转为引用并清除。Windows 已实测：本机 config.json 的 4 个明文实例自动转为 builtin 引用（含备用的新 GLM key）、builtin 凭据拉取与渲染正常。mac 端迁移与拉取链路未实测。
**验证/操作方法**（在 mac 上）：用旧版 config.json（key 明文）+ 新版 secrets.enc 启动，确认明文自动转为 `builtin` 引用、卡片正常出数；`keys.local.json` 数组化后重跑 `node scripts/encrypt-keys.mjs` 能构建。
**完成判据**：与 Windows 行为一致；`config.json` 中无任何 key 明文。

### [ ] 12. 多实例改造的遗留清理（可选，2026-10-20）

**背景**：key 改运行期配置后有两处无害遗留：① 用户数据目录的旧 `balance.json` 不再被读取（程序不主动删用户文件）；② 旧版 config.json 里的 `glmApiKey` / `minimaxApiKey` / `deepseekApiKey` 字段已无消费者（播种走的是编译期内置 key，非这些字段）。
**操作**：手动删除数据目录的 `balance.json`（Windows：`%APPDATA%\aistatus\`；mac：`~/Library/Application Support/aistatus/`）；config.json 旧字段可在下次改配置时顺手删，或不管（serde 读-改-写会原样保留，无副作用）。
**完成判据**：删除后应用运行正常、无异常日志。

### [ ] 4. （可选）重装 rustup，根治 cargo PATH stub

**背景**：本机 `~/.cargo/bin` 下 rustup 代理 stub 缺失，`cargo` 不在 PATH。`scripts/build.mjs` 已兜底，日常构建不受影响；此项为根治。
**操作**：见 `CLAUDE.md`「构建与打包 → 构建环境前提 → 根治」（PowerShell 跑 `rustup-init -y --default-toolchain stable-x86_64-pc-windows-msvc`）。
**完成判据**：新开终端 `where cargo` 能命中 `~/.cargo/bin/cargo.exe`。

### [ ] 5. （视情况）卸载旧 identifier 的安装版

**背景**：identifier 从 `com.aistatus.app` 改为 `com.aistatus.desktop`，新版是不同 Windows 产品，不会覆盖旧版。
**操作**：若之前装过旧版 setup，到「设置 → 应用」卸载 `AI余额监控`（旧 product code），避免两份共存。
**完成判据**：系统只剩 `com.aistatus.desktop` 一份。

### [ ] 7. （可选）MiMo Cookie 失效的托盘气泡提醒

**背景**：2026-08-26 接入小米 MiMo Token Plan 用量查询（运行期 Cookie 配置）。Cookie 过期后目前仅在卡片内显示「Cookie 已失效」占位条，用户不看主窗口时无感知。
**操作**：`fetcher/mimo.rs` 检测到 401/403 或业务 code 非 0 时，经托盘发一次系统通知（需去重，避免 60s 刷新重复轰炸）。
**完成判据**：Cookie 失效后收到一次托盘通知，且同一失效期内不重复提示。

### [ ] 9. 发布侧：防最终产物被杀软误报（签名 / CI 构建 / 白名单申报）

**背景**：2026-10-14 本机杀软（Avira 系 ML 启发式，报毒名 TR/W64.Evo）确定性误报隔离 `tauri-plugin-opener` 的 build script，已移除该插件（改直接依赖底层 `open` crate）根治本地构建问题（另加 `[profile.dev/release.build-override] debug = false` 减脂）。但发布产物 `aistatus.exe` 同样无签名，「新编译 + 无签名 + 本地构建」在用户机器上被同类 ML 引擎命中只是时间问题。
**操作**（可选组合，发版前做）：① 购买代码签名证书给 release 产物签名（根治）；② 改用 GitHub Actions `windows-latest` 构建 Windows 产物（仓库已有 `mac-check.yml` 先例）；③ 向主流杀软厂商提交文件哈希做误报申报 / 白名单。
**完成判据**：发布 exe 带有效签名，或经多家杀软扫描无误报记录。
