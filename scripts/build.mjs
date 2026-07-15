// 构建 wrapper：确保 cargo/rustc 在 PATH 后再调本地 Tauri CLI。
//
// 背景：本机 ~/.cargo/bin 下的 rustup 代理 stub 缺失，导致 cargo 不在 PATH。
// 这里优先用 ~/.cargo/bin（正常环境），找不到则回退到 toolchain bin（当前环境兜底），
// 让 `pnpm build:tauri` 在两种环境下都能直接跑。根治方案见 CLAUDE.md「构建环境」。
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

const home = os.homedir();
const isWin = process.platform === "win32";
const sep = isWin ? ";" : ":";
const exe = isWin ? ".exe" : "";

const cargoBin = path.join(home, ".cargo", "bin");

// 兜底：rustup stub 缺失时，从 ~/.rustup/toolchains/ 找当前平台的 toolchain bin。
// 多平台：Windows 强制 msvc（gnu 会撞 GNU ld 序号上限），mac/Linux 选对应平台 toolchain。
function findToolchainBin() {
  const dir = path.join(home, ".rustup", "toolchains");
  if (!fs.existsSync(dir)) return null;
  const plat = isWin
    ? "windows-msvc"
    : process.platform === "darwin"
      ? "apple-darwin"
      : "unknown-linux";
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  // 目录名形如 stable-x86_64-pc-windows-msvc / stable-aarch64-apple-darwin
  return (
    fs
      .readdirSync(dir)
      .filter((e) => e.includes(plat) && e.includes(arch))
      .map((e) => path.join(dir, e, "bin"))
      .find((p) => fs.existsSync(path.join(p, `cargo${exe}`))) ?? null
  );
}

const env = { ...process.env };
// Tauri 的 beforeBuildCommand 由系统 shell 执行；确保它能找到当前运行
// 此脚本的 Node 可执行文件，即使 PATH 里只有 PowerShell shim。
const extra = [path.dirname(process.execPath)];
if (fs.existsSync(path.join(cargoBin, `cargo${exe}`))) {
  extra.push(cargoBin); // 正常：rustup stub 在
} else {
  const toolchainBin = findToolchainBin();
  if (toolchainBin) {
    extra.push(toolchainBin); // 兜底：直接用 toolchain
    console.log(`[build] ~/.cargo/bin 缺 rustup stub，回退到 ${toolchainBin}`);
  }
}
if (extra.length) env.PATH = extra.join(sep) + sep + (env.PATH ?? "");

// 直接运行项目已安装的 Tauri CLI，避免 Windows 上只有 pnpm.ps1 时，
// `shell: true` 的 cmd.exe 找不到 pnpm 的问题；Node 路径在三平台一致可用。
const tauriCli = path.join(process.cwd(), "node_modules", "@tauri-apps", "cli", "tauri.js");
if (!fs.existsSync(tauriCli)) {
  console.error("[build] 找不到本地 Tauri CLI，请先运行 pnpm install");
  process.exit(1);
}

const r = spawnSync(process.execPath, [tauriCli, "build", ...process.argv.slice(2)], {
  stdio: "inherit",
  env,
});
process.exit(r.status ?? 1);
