import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri 期望的固定 dev 端口；devUrl 在 tauri.conf.json 里对齐。
// clearScreen:false 让 Rust 编译错误不被 Vite 清屏清掉。
// port:1420 + strictPort 保证 Tauri 启动 webview 时一定能连上。
export default defineConfig({
  plugins: [svelte()],
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
