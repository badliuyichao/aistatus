// 进度条配色：悬浮条与主看板共用，保证两处颜色语义一致。
//
// 设计：先按服务分冷暖组，再按周期分色相。
//   GLM（冷色组）    5小时 电光蓝 #3B82F6 / 周 翡翠绿 #34D399
//   MiniMax（暖色组）5小时 亮洋红 #D946EF / 周 珊瑚橙 #FB923C
//   MiMo             单条，品牌橙 #FF6900
// 未知服务回退到服务品牌色。用量 ≥80% 时统一转红告警，
// 其余情况颜色不随用量变化（避免两条同时变黄无法区分）。
//
// 填充仍用同色相「浅→深」渐变：渐变按整条 track 铺开（gradientSize），
// 填充越宽露出的颜色越深。渐变字符串在 JS 里生成，不用 color-mix
// （Safari 16.4 / 老 WebView2 不支持 color-mix）。

import type { ServiceInfo } from "./types";

export type BarPeriod = "5h" | "week";

export interface BarService {
  vendor?: string;
  color?: string;
}

/** 配额项 label → 周期：含「周」的按本周处理，其余（含 MiMo Token Plan）按单条处理。 */
export function periodOfLabel(label: string): BarPeriod {
  return label.includes("周") ? "week" : "5h";
}

/** 进度条固定底色：按服务商 + 周期取色相；未知服务商回退下发品牌色。
 *  同品牌多实例共用色相，实例之间靠名称标签区分（颜色编码品牌 + 周期）。 */
export function barAccent(svc: BarService | undefined, period: BarPeriod = "5h"): string {
  const vendor = svc?.vendor ?? "";
  if (vendor === "glm") {
    return period === "week" ? "#34D399" : "#3B82F6";
  }
  if (vendor === "minimax") {
    return period === "week" ? "#FB923C" : "#D946EF";
  }
  if (vendor === "mimo") return "#FF6900";
  return svc?.color || "#22c55e";
}

/** 进度条底色：服务固定色；≥80% 将耗尽时统一转红告警。 */
export function barColor(
  pct: number | null,
  svc: BarService | undefined,
  period: BarPeriod = "5h"
): string {
  if (pct === null) return "var(--bar-track)"; // ∞ / N/A：灰色占位
  if (pct >= 80) return "#ef4444"; // 红：将耗尽
  return barAccent(svc, period);
}

/** 把 #rgb / #rrggbb 朝 target 色混合，ratio 是 target 的占比（0–1）。 */
function mixColor(hex: string, target: string, ratio: number): string {
  const parse = (h: string): [number, number, number] | null => {
    const m = h.trim().match(/^#?([0-9a-f]{3}|[0-9a-f]{6})$/i);
    if (!m) return null;
    let s = m[1];
    if (s.length === 3) s = s.split("").map((c) => c + c).join("");
    return [
      parseInt(s.slice(0, 2), 16),
      parseInt(s.slice(2, 4), 16),
      parseInt(s.slice(4, 6), 16),
    ];
  };
  const a = parse(hex);
  const b = parse(target);
  if (!a || !b) return hex;
  return (
    "#" +
    a
      .map((v, i) => Math.round(v + (b[i] - v) * ratio).toString(16).padStart(2, "0"))
      .join("")
  );
}

/**
 * 同色相浅→深渐变：起点是掺白的品牌色，终点在基础色上压暗。
 * 白混合比例取 0.55：低用量时也能看清是哪个服务的颜色（0.72 会淡到发灰）。
 */
export function barGradient(
  pct: number,
  svc: BarService | undefined,
  period: BarPeriod = "5h"
): string {
  const base = barColor(pct, svc, period);
  return `linear-gradient(90deg, ${mixColor(base, "#ffffff", 0.55)}, ${mixColor(base, "#000000", 0.22)})`;
}

/** 让渐变图按整条 track 的宽度铺开，而不是按当前填充宽度压缩。 */
export function gradientSize(pct: number): string {
  const size = pct > 0 ? 10000 / pct : 100;
  return `${Math.round(size * 100) / 100}% 100%`;
}
