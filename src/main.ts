import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";

// 全局错误捕获：任何挂载前的 JS 错误直接写到页面上，方便在 GUI 环境调试
// （webview console 不易查看）。挂载成功后此 fallback 会被覆盖。
window.addEventListener("error", (e) => {
  const div = document.createElement("pre");
  div.style.cssText =
    "color:red;font-size:12px;padding:8px;white-space:pre-wrap;background:white;";
  div.textContent = "JS Error: " + (e.error?.stack || e.message);
  document.body.appendChild(div);
});

window.addEventListener("unhandledrejection", (e) => {
  const div = document.createElement("pre");
  div.style.cssText =
    "color:red;font-size:12px;padding:8px;white-space:pre-wrap;background:white;";
  div.textContent = "Promise Rejection: " + (e.reason?.stack || String(e.reason));
  document.body.appendChild(div);
});

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
