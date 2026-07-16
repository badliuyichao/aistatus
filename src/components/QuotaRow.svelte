<script lang="ts">
  // 配额行：标签 + 已用/总量 + 进度条 + 详情。
  import type { QuotaItem } from "../types";
  import { percentage } from "../types";

  let { item, color }: { item: QuotaItem; color: string } = $props();

  let pct = $derived(percentage(item));
  let valueText = $derived(
    item.displayValue ?? `${fmtNum(item.used)}/${fmtNum(item.total)} ${item.unit}`
  );

  // used/total 显示：整数不带小数，浮点保留一位。
  function fmtNum(n: number): string {
    return Number.isInteger(n) ? String(n) : n.toFixed(1);
  }
</script>

<div class="row">
  <div class="label-row">
    <span class="name">{item.label}</span>
    <span
      class="value"
      class:unlimited={item.displayValue === "∞"}
      title={item.displayValue === "∞" ? "无限额" : undefined}
    >{valueText}</span>
  </div>
  <div class="bar" style="--pct:{pct}%; --color:{color};"></div>
  {#if item.detail}
    <div class="detail">{item.detail}</div>
  {/if}
</div>

<style>
  .row {
    padding: 4px 0;
  }
  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 6px;
  }
  .name {
    color: var(--text-primary);
    font-size: 12px;
  }
  .value {
    color: var(--text-secondary);
    font-size: 11px;
  }
  .value.unlimited {
    font-size: 17px;
    line-height: 12px;
  }
  .bar {
    height: 8px;
    margin-top: 4px;
    border-radius: 4px;
    background: var(--bar-track);
    position: relative;
    overflow: hidden;
  }
  .bar::after {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    height: 100%;
    width: var(--pct);
    border-radius: 4px;
    /* 直接用品牌色（旧实现用 color-mix 做半透明渐变，但 color-mix 需 Safari 16.4+，
       macOS 12 / 老 WebView2 不支持，会导致进度条填充透明不可见）。纯色全平台兼容。 */
    background: var(--color);
    transition: width 0.3s ease;
  }
  .detail {
    color: var(--text-muted);
    font-size: 10px;
    margin-top: 4px;
  }
</style>
