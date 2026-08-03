// 封装 Tauri IPC，给组件一个干净的接口。
// 封装前端调用后端命令与订阅后端事件的入口。

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BalanceData, Theme } from "./types";

// ── commands（前端 → Rust）──

export function getServices(): Promise<BalanceData> {
  return invoke<BalanceData>("get_services");
}

export function refreshNow(): Promise<void> {
  return invoke("refresh_now");
}

export function openBalanceFile(): Promise<void> {
  return invoke("open_balance_file");
}

export function getTheme(): Promise<Theme> {
  return invoke<Theme>("get_theme");
}

export function setTheme(theme: Theme): Promise<void> {
  // 后端 command 函数名为 save_theme，Tauri 按函数名注册，故 invoke 用 save_theme。
  return invoke("save_theme", { theme });
}

// 窗口高度自适应内容：前端量得内容高度后调用，后端据此 set_size 并重新锚定右下角。
export function fitToContent(height: number): Promise<void> {
  return invoke("fit_to_content", { height });
}

// 保存悬浮球窗口位置（逻辑像素）。拖拽结束时落盘，下次启动恢复。
export function saveFloatPosition(x: number, y: number): Promise<void> {
  return invoke("save_float_position", { x, y });
}

// ── events（Rust → 前端）──

// 订阅后端的 services-updated 事件；返回取消订阅函数。
export function onServicesUpdated(
  cb: (data: BalanceData) => void
): Promise<UnlistenFn> {
  return listen<BalanceData>("services-updated", (e) => cb(e.payload));
}
