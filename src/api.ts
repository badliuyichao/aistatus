// 封装 Tauri IPC，给组件一个干净的接口。
// 封装前端调用后端命令与订阅后端事件的入口。

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BalanceData, PlanConfig, Theme } from "./types";

// ── commands（前端 → Rust）──

export function getServices(): Promise<BalanceData> {
  return invoke<BalanceData>("get_services");
}

export function refreshNow(): Promise<void> {
  return invoke("refresh_now");
}

// 用系统默认编辑器打开 config.json（Coding Plan 实例 / 主题等用户配置）。
export function openConfigFile(): Promise<void> {
  return invoke("open_config_file");
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

// ── MiMo Cookie（运行期配置，设置对话框用）──

// 读当前 Cookie（空串 = 未配置），对话框打开时回显。
export function getMimoCookie(): Promise<string> {
  return invoke<string>("get_mimo_cookie");
}

// 保存 Cookie（后端宽容清洗 + 持久化 + 立即触发刷新验证）。
// 传空串表示清除配置。
export function setMimoCookie(cookie: string): Promise<void> {
  return invoke("save_mimo_cookie", { cookie });
}

// ── Coding Plan 实例管理（设置对话框用）──

// 读 plan 实例列表（含完整 key，仅用于本地回显，界面只展示尾部指纹）。
export function getPlans(): Promise<PlanConfig[]> {
  return invoke<PlanConfig[]>("get_plans");
}

// 保存 plan 实例列表（至少保留一个实例，后端校验；缺失 id 由后端生成）。
// 返回后端规范化后的权威列表（前端据此同步新增实例的 id）。
// 保存后显隐 / 增删立即生效（后端重发快照），并触发一次刷新补数据。
export function savePlans(plans: PlanConfig[]): Promise<PlanConfig[]> {
  return invoke<PlanConfig[]>("save_plans", { plans });
}

// ── events（Rust → 前端）──

// 订阅后端的 services-updated 事件；返回取消订阅函数。
export function onServicesUpdated(
  cb: (data: BalanceData) => void
): Promise<UnlistenFn> {
  return listen<BalanceData>("services-updated", (e) => cb(e.payload));
}
