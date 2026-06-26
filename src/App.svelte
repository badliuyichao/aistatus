<script lang="ts">
  // 主壳：标题栏 + 卡片列表 + 页脚 + 右键菜单 + F5 + 首次引导。
  // 对应 legacy widget.py 的 MainWidget。
  import { onMount, onDestroy } from "svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import ServiceCard from "./components/ServiceCard.svelte";
  import ConfigDialog from "./components/ConfigDialog.svelte";
  import { services } from "./stores/services";
  import { needsKeySetup, openBalanceFile, refreshNow } from "./api";
  import { formatTime } from "./types";

  let showConfig = $state(false);
  let showMenu = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);

  onMount(async () => {
    await services.init();
    // 首次无 key → 弹配置框（对应 legacy prompt_for_keys_if_needed）
    try {
      if (await needsKeySetup()) {
        showConfig = true;
      }
    } catch (e) {
      console.error("needs_key_setup failed", e);
    }
  });

  onDestroy(() => services.destroy());

  // F5 刷新（对应 legacy QShortcut F5）
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "F5") {
      e.preventDefault();
      services.refresh();
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    menuX = e.clientX;
    menuY = e.clientY;
    showMenu = true;
  }

  async function refresh() {
    showMenu = false;
    await services.refresh();
  }

  async function editData() {
    showMenu = false;
    try {
      await openBalanceFile();
    } catch (e) {
      console.error("open_balance_file failed", e);
    }
  }

  function openConfig() {
    showMenu = false;
    showConfig = true;
  }
</script>

<svelte:window onkeydown={onKeydown} />
{#if showMenu}
  <!-- 点其他地方关闭菜单 -->
  <div
    class="menu-backdrop"
    role="presentation"
    onclick={() => (showMenu = false)}
    oncontextmenu={(e) => {
      e.preventDefault();
      showMenu = false;
    }}
  ></div>
{/if}

<div class="app" role="application" oncontextmenu={onContextMenu}>
  <div class="bg-card">
    <TitleBar title={services.data.title} onconfig={openConfig} />
    <div class="separator"></div>

    <div class="scroll">
      <div class="cards">
        {#if services.data.services.length === 0}
          <div class="empty">暂无数据<br /><br />请编辑 balance.json 添加服务</div>
        {:else}
          {#each services.data.services as svc (svc.name)}
            <ServiceCard service={svc} />
          {/each}
        {/if}
      </div>
    </div>

    <div class="footer">
      {#if services.refreshing}刷新中…{:else}⏱ 更新于 {formatTime(services.data.timestamp)}{/if}
    </div>
  </div>
</div>

{#if showMenu}
  <div class="ctx-menu" style="left:{menuX}px; top:{menuY}px">
    <button onclick={refresh}>刷新数据</button>
    <button onclick={editData}>编辑数据文件</button>
    <button onclick={openConfig}>配置 API Key</button>
  </div>
{/if}

{#if showConfig}
  <ConfigDialog onclose={() => (showConfig = false)} />
{/if}

<style>
  .app {
    height: 100%;
    overflow: hidden;
  }
  .bg-card {
    height: 100%;
    background: var(--card-bg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .separator {
    height: 1px;
    background: rgba(0, 0, 0, 0.08);
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .scroll::-webkit-scrollbar {
    width: 4px;
  }
  .scroll::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.18);
    border-radius: 2px;
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 12px;
  }
  .empty {
    color: #586070;
    font-size: 12px;
    text-align: center;
    padding: 40px;
    line-height: 1.6;
  }
  .footer {
    color: #9aa1ad;
    font-size: 10px;
    text-align: center;
    padding: 6px 14px;
  }
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
  }
  .ctx-menu {
    position: fixed;
    z-index: 60;
    background: #2a2a3a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 4px;
    font-size: 12px;
    color: #ddd;
    min-width: 140px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
  }
  .ctx-menu button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 16px;
    border: none;
    background: transparent;
    color: #ddd;
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
  }
  .ctx-menu button:hover {
    background: rgba(255, 255, 255, 0.1);
    color: white;
  }
</style>
