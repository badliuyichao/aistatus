// DeepSeek 余额状态分级。
//
// 阈值按人民币余额的经验值定：≥50 安全 / 10–<50 预警 / <10 危险。
// USD 等外币账户按数值直接套用（未折算汇率），只会偏严，不影响功能。
// 错误占位（key 无效 / 服务繁忙）由 ServiceInfo.placeholder 标记，
// 调用方应先排除 placeholder，再用金额判断状态。

export type BalanceStatus = {
  label: "安全" | "预警" | "危险";
  color: string;
};

export function balanceStatus(balance: number): BalanceStatus {
  if (balance < 10) return { label: "危险", color: "#ef4444" }; // 红
  if (balance < 50) return { label: "预警", color: "#f59e0b" }; // 黄
  return { label: "安全", color: "#22c55e" }; // 绿
}
