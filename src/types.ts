// 与 Rust src-tauri/src/data.rs 的 serde 输出对齐（camelCase）。
// 后端 emit('services-updated', BalanceData) 和 invoke('get_services') 都返回这些类型。

export interface QuotaItem {
  label: string;
  displayValue?: string; // 自定义右侧显示，如无限额 ∞
  used: number;
  total: number;
  unit: string;
  detail: string;
}

export type ServiceType = "quota" | "balance";

// 主题偏好，对应后端 config.json 的 theme 字段。
// system 由前端用 matchMedia 解析成 dark/light。
export type Theme = "system" | "dark" | "light";

export interface ServiceInfo {
  name: string;
  type: ServiceType;
  icon: string;
  color: string;
  items: QuotaItem[]; // quota 型
  balance: number; // balance 型
  currency: string;
  unit: string;
  detail: string;
  loading: boolean; // 无实时数据时卡片显示 N/A
}

export interface BalanceData {
  title: string;
  services: ServiceInfo[];
  timestamp?: number; // Unix 秒
}

// get_keys / save_keys 用
export interface ApiKeys {
  glmApiKey: string;
  minimaxApiKey: string;
}

// 工具：已用百分比（与 Rust QuotaItem::percentage 一致）
export function percentage(item: QuotaItem): number {
  if (item.total <= 0) return 0;
  return Math.min((item.used / item.total) * 100, 100);
}

// 工具：把 Unix 秒格式化为「YYYY-MM-DD HH:MM:SS」。
export function formatTime(ts?: number): string {
  if (!ts) return "";
  const d = new Date(ts * 1000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(
    d.getHours()
  )}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
}
