# TODO — 待确认 / 待办事项

> 本次（2026-06-30）改动中，**本机为 Windows、无法在此验证** 的事项，以及相关后续待办。
> 用法与格式约定见 `CLAUDE.md`「TODO 清单（todo.md）」。

---

## 🔬 无法在本机确认（需对应平台验证）

### [ ] 1. macOS 编译验证：`lib.rs` 的条件编译改动

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

### [ ] 2. macOS / Linux：`build.mjs` 的 `findToolchainBin` 兜底实测

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

---

## 🧹 其他待办（非阻塞）

### [ ] 3. （可选）重装 rustup，根治 cargo PATH stub

**背景**：本机 `~/.cargo/bin` 下 rustup 代理 stub 缺失，`cargo` 不在 PATH。`scripts/build.mjs` 已兜底，日常构建不受影响；此项为根治。
**操作**：见 `CLAUDE.md`「构建与打包 → 构建环境前提 → 根治」（PowerShell 跑 `rustup-init -y --default-toolchain stable-x86_64-pc-windows-msvc`）。
**完成判据**：新开终端 `where cargo` 能命中 `~/.cargo/bin/cargo.exe`。

### [ ] 4. （视情况）卸载旧 identifier 的安装版

**背景**：identifier 从 `com.aistatus.app` 改为 `com.aistatus.desktop`，新版是不同 Windows 产品，不会覆盖旧版。
**操作**：若之前装过旧版 setup，到「设置 → 应用」卸载 `AI余额监控`（旧 product code），避免两份共存。
**完成判据**：系统只剩 `com.aistatus.desktop` 一份。
