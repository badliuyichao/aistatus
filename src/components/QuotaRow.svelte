<script lang="ts">
  // 配额行：标签 + 已用/总量 + 进度条 + 详情。
  // 对应 legacy widget.py 的 quota row（_make_quota_row）+ QuotaProgressBar。
  import type { QuotaItem } from "../types";
  import { percentage } from "../types";

  let { item, color }: { item: QuotaItem; color: string } = $props();

  let pct = $derived(percentage(item));

  // used/total 显示：整数不带小数，浮点保留一位（对应 legacy 的判断逻辑）
  function fmtNum(n: number): string {
    return Number.isInteger(n) ? String(n) : n.toFixed(1);
  }
</script>

<div class="row">
  <div class="label-row">
    <span class="name">{item.label}</span>
    <span class="value">{fmtNum(item.used)}/{fmtNum(item.total)} {item.unit}</span>
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
    color: #2d3340;
    font-size: 12px;
  }
  .value {
    color: #586070;
    font-size: 11px;
  }
  .bar {
    height: 8px;
    margin-top: 4px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.06);
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
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--color) 86%, transparent),
      color-mix(in srgb, var(--color) 70%, transparent)
    );
    transition: width 0.3s ease;
  }
  .detail {
    color: #8a92a0;
    font-size: 10px;
    margin-top: 4px;
  }
</style>
