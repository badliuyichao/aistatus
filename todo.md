# TODO — 待确认 / 待办事项

> 本次（2026-06-30）改动中，**本机为 Windows、无法在此验证** 的事项，以及相关后续待办。
> 用法与格式约定见 `CLAUDE.md`「TODO 清单（todo.md）」。

---

## 🔬 无法在本机确认（需对应平台验证）

### [ ] 1. macOS / Linux：验证内嵌 balance.json 首次启动模板

**背景**：已移除旧实现目录，并改为通过 Rust `include_str!` 将 `src-tauri/resources/balance.json` 编译进程序，避免运行时依赖安装目录外的文件。本机仅能验证 Windows。

**验证/操作方法**：在 macOS 或 Linux 上构建应用，删除该系统用户配置目录中的 `aistatus/balance.json` 后首次启动；再通过「编辑数据文件」确认文件已生成且可正常打开。

**完成判据**：首次启动后在对应系统配置目录生成有效的 `balance.json`，界面正常显示默认服务卡片。

### [ ] 2. macOS 编译验证：`lib.rs` 的条件编译改动

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

---

## 🧹 其他待办（非阻塞）

### [ ] 4. （可选）重装 rustup，根治 cargo PATH stub

**背景**：本机 `~/.cargo/bin` 下 rustup 代理 stub 缺失，`cargo` 不在 PATH。`scripts/build.mjs` 已兜底，日常构建不受影响；此项为根治。
**操作**：见 `CLAUDE.md`「构建与打包 → 构建环境前提 → 根治」（PowerShell 跑 `rustup-init -y --default-toolchain stable-x86_64-pc-windows-msvc`）。
**完成判据**：新开终端 `where cargo` 能命中 `~/.cargo/bin/cargo.exe`。

### [ ] 5. （视情况）卸载旧 identifier 的安装版

**背景**：identifier 从 `com.aistatus.app` 改为 `com.aistatus.desktop`，新版是不同 Windows 产品，不会覆盖旧版。
**操作**：若之前装过旧版 setup，到「设置 → 应用」卸载 `AI余额监控`（旧 product code），避免两份共存。
**完成判据**：系统只剩 `com.aistatus.desktop` 一份。
