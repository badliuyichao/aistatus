<script lang="ts">
  // 主壳：标题区 + 卡片列表 + 页脚 + 右键菜单 + F5 + 首次引导。
  // 对应 legacy widget.py 的 MainWidget。
  // macOS 用原生红绿灯（关闭/最小化/全屏），窗口控制不再用自定义按钮；
  // 配置入口移到右键菜单。窗口默认 alwaysOnTop（见 tauri.conf.json）。
  import { onMount, onDestroy, tick } from "svelte";
  import ServiceCard from "./components/ServiceCard.svelte";
  import ConfigDialog from "./components/ConfigDialog.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import { services } from "./stores/services.svelte";
  import { theme } from "./stores/theme.svelte";
  import { needsKeySetup, openBalanceFile, fitToContent } from "./api";
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
  let bgEl: HTMLDivElement | undefined = $state();
  let cardsEl: HTMLDivElement | undefined = $state();
  let resizeObserver: ResizeObserver | undefined;

  // 测量内容「自然高度」并通知后端把窗口高度自适应到卡片数量。
  // 注意：不能量 .bg-card.scrollHeight——它是 height:100% + .scroll flex:1 撑满，
  // scrollHeight 会回到窗口高度（=当前 680），形成循环量不出真实内容。
  // 正确做法：把各部分的「自然高度」相加 = 标题栏 + 分隔线 + 卡片区(.scroll 的
  // scrollHeight，即 .cards 完整高度) + 页脚；超出后端上限时窗口不再长高、滚动兜底。
  function measureAndFit() {
    const root = bgEl;
    const cards = cardsEl;
    if (!root || !cards) return;
    const titlebar = root.querySelector<HTMLElement>(".titlebar");
    const separator = root.querySelector<HTMLElement>(".separator");
    const footer = root.querySelector<HTMLElement>(".footer");
    if (!titlebar || !separator || !footer) return;
    // 必须用 .cards 的 offsetHeight（自然高度）。不能用 .bg-card / .scroll 的
    // scrollHeight——它们是 height:100% / flex:1，scrollHeight = max(自身高度, 内容)
    // ≈ 当前窗口高度，会循环量不出真实内容。
    const h =
      titlebar.offsetHeight +
      separator.offsetHeight +
      cards.offsetHeight +
      footer.offsetHeight;
    if (h > 0) {
      fitToContent(h).catch((e) => console.error("fit_to_content failed", e));
    }
  }

  onMount(async () => {
    // 主题先于 services 初始化：读偏好 + 应用 <html data-theme>。
    // 窗口默认隐藏，唤起前完成，无主题闪烁。
    await theme.init();
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

    // 窗口高度自适应卡片数量：渲染就绪后量一次，再用 ResizeObserver 跟踪
    // 增删服务 / 文字换行等后续变化（卡片内容高度变化才触发，不会因窗口自身缩放循环）
    await tick();
    measureAndFit();
    if (cardsEl) {
      resizeObserver = new ResizeObserver(() => measureAndFit());
      resizeObserver.observe(cardsEl);
    }
  });

  onDestroy(() => {
    services.destroy();
    theme.destroy();
    unlistenOpenConfig?.();
    resizeObserver?.disconnect();
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
  <div class="bg-card" bind:this={bgEl}>
    <div class="titlebar" class:mac={isMac}><TitleBar /></div>
    <div class="separator"></div>

    <div class="scroll">
      <div class="cards" bind:this={cardsEl}>
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
      <div class="footer-status">
        {#if services.refreshing}刷新中…{:else}⏱ 更新于 {formatTime(services.data.timestamp)}{/if}
      </div>
      <div class="footer-version">v{__APP_VERSION__} · {__BUILD_TIME__} · {__GIT_HASH__}</div>
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
    /* 标题栏：承载 TitleBar 的拖拽区 + 平台窗口控制按钮。
       mac 红绿灯由 .titlebar.mac 的 padding-left 让位；Windows 按钮贴右上角。 */
    height: 28px;
    min-height: 28px;
    display: flex;
    align-items: stretch;
    padding: 0;
  }
  .titlebar.mac {
    /* macOS 原生红绿灯(Overlay)占据左上角，留出空间避免遮挡下方内容 */
    padding-left: 78px;
  }
  .separator {
    height: 1px;
    background: var(--separator);
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
    background: var(--scroll-thumb);
    border-radius: 2px;
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 12px;
  }
  .empty {
    color: var(--text-secondary);
    font-size: 12px;
    text-align: center;
    padding: 40px;
    line-height: 1.6;
  }
  .footer {
    color: var(--text-muted);
    font-size: 10px;
    text-align: center;
    padding: 6px 14px;
  }
  .footer-version {
    margin-top: 2px;
    opacity: 0.7;
    font-size: 9px;
  }
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
  }
  .ctx-menu {
    position: fixed;
    z-index: 60;
    background: var(--menu-bg);
    border: 1px solid var(--menu-border);
    border-radius: 8px;
    padding: 4px;
    font-size: 12px;
    color: var(--menu-text);
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
    color: var(--menu-text);
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
  }
  .ctx-menu button:hover {
    background: var(--menu-hover);
    color: white;
  }
</style>
