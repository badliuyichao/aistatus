<script lang="ts">
  // 主壳：标题区 + 卡片列表 + 页脚 + 右键菜单 + F5 + 首次引导。
  // 对应 legacy widget.py 的 MainWidget。
  // macOS 用原生红绿灯（关闭/最小化/全屏），窗口控制不再用自定义按钮；
  // 配置入口移到右键菜单。窗口默认 alwaysOnTop（见 tauri.conf.json）。
  import { onMount, onDestroy } from "svelte";
  import ServiceCard from "./components/ServiceCard.svelte";
  import ConfigDialog from "./components/ConfigDialog.svelte";
  import { services } from "./stores/services.svelte";
  import { needsKeySetup, openBalanceFile } from "./api";
  import { listen } from "@tauri-apps/api/event";
  import { formatTime } from "./types";

  const isMac =
    typeof navigator !== "undefined" &&
    /mac/i.test(navigator.userAgent || navigator.platform);

  let showConfig = $state(false);
  let showMenu = $state(false);
  let menuX = $state(0);
  let menuY = $state(0);
  let menuEl: HTMLDivElement | undefined = $state();
  let unlistenOpenConfig: (() => void) | null = null;

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
    // 托盘菜单「设置」→ 后端发 open-config 事件，前端打开配置对话框
    unlistenOpenConfig = await listen("open-config", () => {
      showConfig = true;
    });
  });

  onDestroy(() => {
    services.destroy();
    unlistenOpenConfig?.();
  });

  // F5 刷新（对应 legacy QShortcut F5）
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "F5") {
      e.preventDefault();
      services.refresh();
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    // 初始用点击坐标，随后由 $effect 钳制到视口内防止溢出被裁剪
    menuX = e.clientX;
    menuY = e.clientY;
    showMenu = true;
  }

  // 菜单显示后测量尺寸并钳制，避免靠近右/下边界时溢出窗口被裁剪
  $effect(() => {
    if (!showMenu || !menuEl) return;
    const w = menuEl.offsetWidth;
    const h = menuEl.offsetHeight;
    menuX = Math.min(menuX, window.innerWidth - w - 4);
    menuY = Math.min(menuY, window.innerHeight - h - 4);
    menuX = Math.max(4, menuX);
    menuY = Math.max(4, menuY);
  });

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
    <div class="titlebar" class:mac={isMac} data-tauri-drag-region></div>
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
  <div class="ctx-menu" bind:this={menuEl} style="left:{menuX}px; top:{menuY}px">
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
  .titlebar {
    /* 仅作为窗口拖拽区 + mac 红绿灯占位，无文字，保持低高度清爽 */
    height: 28px;
    min-height: 28px;
    display: flex;
    align-items: center;
    padding: 0 12px;
  }
  .titlebar.mac {
    /* macOS 原生红绿灯(Overlay)占据左上角，留出空间避免遮挡下方内容 */
    padding-left: 78px;
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
