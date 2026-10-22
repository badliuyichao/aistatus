<script lang="ts">
  // 配额行：标签 + 已用/总量 + 进度条 + 详情。
  // 进度条配色与悬浮条共用 barColors（按服务 + 周期分色）。
  import type { QuotaItem, ServiceInfo } from "../types";
  import { percentage } from "../types";
  import { barGradient, gradientSize, periodOfLabel } from "../barColors";

  let { item, service }: { item: QuotaItem; service: ServiceInfo } = $props();

  let pct = $derived(percentage(item));
  let infinite = $derived(item.displayValue === "∞");
  let period = $derived(periodOfLabel(item.label));
  // 渐变串与展开尺寸放进 CSS 变量，供伪元素 ::after 使用。
  let barVars = $derived(
    `--pct:${pct}%; --gradient:${barGradient(pct, service, period)}; --size:${gradientSize(pct)};`
  );
  // 百分比条（unit=="%"）右侧只显示「N%」——「0/100 %」的比值写法无信息量。
  let valueText = $derived(
    item.displayValue ??
      (item.unit === "%"
        ? `${Math.round(pct)}%`
        : `${fmtNum(item.used)}/${fmtNum(item.total)} ${item.unit}`)
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
  <div class="bar" class:infinite={infinite} style={barVars}></div>
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
    /* 渐变串由 barColors 在 JS 里生成，避免 color-mix（Safari 16.4 / 老 WebView2 不支持）；
       background-size 让渐变按整条 track 铺开，填充越宽露出的颜色越深。 */
    background-image: var(--gradient);
    background-size: var(--size);
    background-repeat: no-repeat;
    transition: width 0.3s ease;
  }
  /* ∞ 不限额：满格中性色斜纹，与「100% 已用」的同色相深色区分（同悬浮条）。 */
  .bar.infinite::after {
    width: 100%;
    background-image: repeating-linear-gradient(
      45deg,
      rgba(180, 190, 210, 0.5),
      rgba(180, 190, 210, 0.5) 4px,
      rgba(140, 150, 170, 0.5) 4px,
      rgba(140, 150, 170, 0.5) 8px
    );
    background-size: auto;
  }
  .detail {
    color: var(--text-muted);
    font-size: 10px;
    margin-top: 4px;
  }
</style>
