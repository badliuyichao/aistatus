<script lang="ts">
  // 服务卡片：图标 + 名称 + 状态点 + 内容（quota 多行 / balance 金额）。
  // 对应 legacy widget.py 的 ServiceCard。
  import type { ServiceInfo } from "../types";
  import QuotaRow from "./QuotaRow.svelte";
  import BalanceView from "./BalanceView.svelte";

  // 品牌图标按名称前缀匹配（对应 legacy _service_logo 的 startswith 逻辑）
  import glmLogo from "../assets/glm.png";
  import deepseekLogo from "../assets/deepseek.png";
  import minimaxLogo from "../assets/minimax.png";

  let { service }: { service: ServiceInfo } = $props();

  function logo(name: string): string | null {
    const n = (name || "").trim();
    if (n.startsWith("GLM")) return glmLogo;
    if (n.startsWith("DeepSeek")) return deepseekLogo;
    if (n.startsWith("MiniMax")) return minimaxLogo;
    return null;
  }

  let brandLogo = $derived(logo(service.name));
  // 状态点颜色：loading 时暗灰，否则主题色（对应 legacy _update_header）
  let statusColor = $derived(service.loading ? "#C8CDD6" : service.color);
</script>

<div class="card" style="--color:{service.color}">
  <div class="header">
    <div class="icon">
      {#if brandLogo}
        <img src={brandLogo} alt={service.name} />
      {:else}
        <span class="emoji">{service.icon || "●"}</span>
      {/if}
    </div>
    <span class="name">{service.name}</span>
    <span class="status-dot" style="background:{statusColor}"></span>
  </div>

  <div class="content">
    {#if service.type === "quota"}
      {#if service.loading && service.items.length === 0}
        <div class="loading-row">—</div>
      {:else}
        {#each service.items as item (item.label)}
          <QuotaRow {item} color={service.color} />
        {/each}
      {/if}
    {:else}
      <BalanceView {service} />
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--svc-card-bg);
    border: 1px solid var(--svc-card-border);
    border-radius: 10px;
    padding: 12px 14px;
  }
  .header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }
  .icon {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .icon img {
    width: 22px;
    height: 22px;
    object-fit: contain;
  }
  .emoji {
    font-size: 18px;
  }
  .name {
    flex: 1;
    color: var(--color);
    font-size: 14px;
    font-weight: 700;
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 4px;
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .loading-row {
    color: var(--text-muted);
    font-size: 12px;
    padding: 4px 0;
  }
</style>
