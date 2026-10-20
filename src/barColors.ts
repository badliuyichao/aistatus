// 进度条配色：悬浮条与主看板共用，保证两处颜色语义一致。
//
// 设计：先按服务分冷暖组，再按周期分色相。
//   GLM（冷色组）    5小时 电光蓝 #3B82F6 / 周 翡翠绿 #34D399
//   MiniMax（暖色组）5小时 亮洋红 #D946EF / 周 珊瑚橙 #FB923C
//   MiMo             单条，品牌橙 #FF6900
// 未知服务回退到服务品牌色。
//
// 仅 GLM 的 5 小时条是固定色带：淡蓝 → 品牌蓝 → 蓝紫 → 深红
// #B91C1C 的四段渐变（中间锚点避免直接蓝→红的灰紫塌陷），配合
// gradientSize 按填充宽度从左往右揭示——低用量只见淡蓝端，越高
// 露出越多红端，满格完整呈现（丝滑过渡，无阈值跳变）。其余条目
// （周限额、MiniMax / MiMo）填充为同色相「浅→深」渐变，底色
// ≥80% 统一转红 #ef4444 告警。
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

/** GLM 5 小时色带右端：深红。 */
const FIVE_H_RAMP_END = "#B91C1C";

/** 进度条底色（非 GLM 色带路径的渐变基色）：服务固定色；
 *  ≥80% 将耗尽时统一转红告警。 */
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

/** GLM 5 小时色带：淡蓝 → 品牌蓝 → 蓝紫 → 深红的四段固定渐变。
 *  单段 RGB 插值从「很浅的蓝」直达「很深的红」会穿过发灰的绛紫
 *  （两端明度落差大、中段饱和度塌陷，观感发脏）；用品牌蓝与蓝紫
 *  做两个中间锚点，每段色相差短、饱和度全程在线，过渡才丝滑。
 *  色带固定、不随 pct 变化——pct 只决定揭示多少（见 gradientSize）。 */
function glmFiveHSpectrum(): string {
  return `linear-gradient(90deg, #a9cdff 0%, #3b82f6 40%, #8b7cf7 72%, ${FIVE_H_RAMP_END} 100%)`;
}

/**
 * 填充渐变。GLM 5 小时条用固定色带（淡蓝→深红，配合 gradientSize
 * 按用量揭示）；其余用同色相「浅→深」渐变：起点掺白 0.55（低用量
 * 也能看清品牌色，0.72 会淡到发灰），终点在底色上压暗 0.22。
 */
export function barGradient(
  pct: number,
  svc: BarService | undefined,
  period: BarPeriod = "5h"
): string {
  if (period === "5h" && svc?.vendor === "glm") {
    return glmFiveHSpectrum();
  }
  const base = barColor(pct, svc, period);
  return `linear-gradient(90deg, ${mixColor(base, "#ffffff", 0.55)}, ${mixColor(base, "#000000", 0.22)})`;
}

/** 让渐变图按整条 track 的宽度铺开，而不是按当前填充宽度压缩。 */
export function gradientSize(pct: number): string {
  const size = pct > 0 ? 10000 / pct : 100;
  return `${Math.round(size * 100) / 100}% 100%`;
}
