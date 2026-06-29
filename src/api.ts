// 封装 Tauri IPC，给组件一个干净的接口。
// 对应 legacy widget 里调用 fetcher / request_refresh 的各种入口。

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ApiKeys, BalanceData } from "./types";

// ── commands（前端 → Rust）──

export function getServices(): Promise<BalanceData> {
  return invoke<BalanceData>("get_services");
}

export function getKeys(): Promise<ApiKeys> {
  return invoke<ApiKeys>("get_keys");
}

export function saveKeys(
  deepseek: string,
  glm: string,
  minimax: string
): Promise<void> {
  return invoke("save_keys", { deepseek, glm, minimax });
}

export function refreshNow(): Promise<void> {
  return invoke("refresh_now");
}

export function needsKeySetup(): Promise<boolean> {
  return invoke<boolean>("needs_key_setup");
}

export function openBalanceFile(): Promise<void> {
  return invoke("open_balance_file");
}

// 窗口高度自适应内容：前端量得内容高度后调用，后端据此 set_size 并重新锚定右下角。
export function fitToContent(height: number): Promise<void> {
  return invoke("fit_to_content", { height });
}

// ── events（Rust → 前端）──

// 订阅后端的 services-updated 事件；返回取消订阅函数。
export function onServicesUpdated(
  cb: (data: BalanceData) => void
): Promise<UnlistenFn> {
  return listen<BalanceData>("services-updated", (e) => cb(e.payload));
}
