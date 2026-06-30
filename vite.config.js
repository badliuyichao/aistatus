import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { createRequire } from "node:module";
import { execSync } from "node:child_process";

// 版本标识（构建时注入前端，见 src/vite-env.d.ts 的全局声明）：
//   - 版本号：取 package.json（单一真相源，由 scripts/sync-version.mjs 同步到
//     tauri.conf.json / Cargo.toml，使产物文件名也带版本）
//   - 构建时间 + git short hash：即使版本号未 bump，也能一眼区分每一次构建产物
const require = createRequire(import.meta.url);
const { version } = require("./package.json");

const now = new Date();
const p = (n) => String(n).padStart(2, "0");
const buildTime = `${p(now.getMonth() + 1)}-${p(now.getDate())} ${p(
  now.getHours()
)}:${p(now.getMinutes())}`;
let gitHash = "dev";
try {
  gitHash = execSync("git rev-parse --short HEAD").toString().trim();
} catch {
  // 无 git 环境时降级为 dev
}

// Tauri 期望的固定 dev 端口；devUrl 在 tauri.conf.json 里对齐。
// clearScreen:false 让 Rust 编译错误不被 Vite 清屏清掉。
// port:1420 + strictPort 保证 Tauri 启动 webview 时一定能连上。
export default defineConfig({
  plugins: [svelte()],
  define: {
    __APP_VERSION__: JSON.stringify(version),
    __BUILD_TIME__: JSON.stringify(buildTime),
    __GIT_HASH__: JSON.stringify(gitHash),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Tauri 用 host 监听；固定到本机，外部访问不到（密钥也不外泄）。
    host: "127.0.0.1",
  },
  // Tauri CLI 设置的 env，用来区分 dev / build 环境。
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // Tauri 用 Objcopy 处理资源，相对路径更稳。
    target: "es2021",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
