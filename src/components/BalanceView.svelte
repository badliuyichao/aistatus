<script lang="ts">
  // 余额展示：大号金额 + 货币符号 + 单位 + 详情。
  // 对应 legacy widget.py 的 _apply_balance / _refill_balance。
  import type { ServiceInfo } from "../types";

  let { service }: { service: ServiceInfo } = $props();
</script>

<div class="balance" style="--color:{service.color}">
  {#if service.loading}
    <span class="amount na">N/A</span>
  {:else}
    {#if service.currency}
      <span class="currency">{service.currency}</span>
    {/if}
    <span class="amount">{service.balance.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
    {#if service.unit}
      <span class="unit">{service.unit}</span>
    {/if}
  {/if}
</div>
{#if service.detail && !service.loading}
  <div class="detail">{service.detail}</div>
{/if}

<style>
  .balance {
    display: flex;
    align-items: baseline;
    gap: 4px;
    padding: 8px 0;
  }
  .currency {
    color: var(--color);
    font-size: 18px;
    font-weight: 600;
  }
  .amount {
    color: var(--color);
    font-size: 36px;
    font-weight: 800;
    line-height: 1;
  }
  .amount.na {
    color: #9aa1ad;
  }
  .unit {
    color: var(--color);
    font-size: 13px;
    margin-bottom: 4px;
  }
  .detail {
    color: #8a92a0;
    font-size: 11px;
    padding-top: 2px;
  }
</style>
