<script lang="ts">
  // 余额展示：大号金额 + 货币符号 + 单位 + 余额状态（安全/预警/危险）+ 详情。
  import type { ServiceInfo } from "../types";
  import { balanceStatus } from "../balanceStatus";

  let { service }: { service: ServiceInfo } = $props();

  // 错误占位（key 无效 / 服务繁忙）的金额是 0，不能据此判状态。
  let status = $derived(
    !service.loading && !service.placeholder ? balanceStatus(service.balance) : null
  );
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
    {#if status}
      <span class="status-badge" style="color:{status.color}; border-color:{status.color}">
        {status.label}
      </span>
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
    color: var(--text-muted);
  }
  .unit {
    color: var(--color);
    font-size: 13px;
    margin-bottom: 4px;
  }
  .status-badge {
    align-self: center;
    margin-left: 2px;
    padding: 2px 7px;
    border: 1px solid currentColor;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
  }
  .detail {
    color: var(--text-muted);
    font-size: 11px;
    padding-top: 2px;
  }
</style>
