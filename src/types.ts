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
  id: string; // 实例稳定 ID（config.json plans 的 id）
  vendor: string; // 服务商稳定 ID：glm / minimax / mimo / deepseek
  name: string; // 展示名（品牌名；GLM 带套餐后缀）
  label: string; // 用户命名的实例标签（可空），区分同品牌实例
  type: ServiceType;
  icon: string;
  color: string;
  items: QuotaItem[]; // quota 型
  balance: number; // balance 型
  currency: string;
  unit: string;
  detail: string;
  loading: boolean; // 无实时数据时卡片显示 N/A
  placeholder?: boolean; // 错误占位（如 API Key 无效），金额不参与状态判断
}

// 一个 Coding Plan 实例（同品牌可配多个），对应 config.json 的 plans 数组。
// glm / minimax / deepseek 用 key 字段；mimo 用 cookie 字段。
// builtin 非空 = 内置实例：凭据编译在二进制里（不落盘），key 字段恒为空，
// 按序号引用该品牌的内置 key；后端序列化时省略该字段。
export interface PlanConfig {
  id: string;
  vendor: string;
  label: string;
  key: string;
  cookie: string;
  visible: boolean;
  builtin?: number | null;
}

export interface BalanceData {
  title: string;
  services: ServiceInfo[];
  timestamp?: number; // Unix 秒
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
