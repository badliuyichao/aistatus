<script lang="ts">
  // 标题栏：拖拽区 + 平台窗口控制按钮。
  //
  // macOS 不渲染任何按钮——原生红绿灯由 titleBarStyle:Overlay 叠在左上角，
  // 父级 .titlebar.mac 已留 padding-left:78px 让位；这里只占满可拖拽区。
  //
  // Windows/Linux 无原生控件（窗口 decorations:false），这里自绘 caption 按钮：
  // 最小化 / 最大化·向下还原 / 关闭，接 Tauri 窗口 API。
  // 「关闭」走 win.close() → 触发后端 on_window_event 的 CloseRequested，
  // 后端 prevent_close + hide（与 mac 红绿灯一致：隐藏到托盘，不退出进程；
  // 真正退出只能托盘菜单「退出」）。
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, onDestroy } from "svelte";

  const isMac =
    typeof navigator !== "undefined" &&
    /mac/i.test(navigator.userAgent || navigator.platform);

  // 仅 Windows/Linux 渲染自绘按钮
  const showControls = !isMac;

  const win = getCurrentWindow();
  let maximized = $state(false);
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    try {
      maximized = await win.isMaximized();
      unlisten = await win.onResized(async () => {
        maximized = await win.isMaximized();
      });
    } catch {
      // 非 Tauri 环境（纯浏览器预览）忽略
    }
  });
  onDestroy(() => unlisten?.());

  const minimize = () => win.minimize();
  const toggleMax = () => win.toggleMaximize();
  const close = () => win.close();
</script>

<!-- 可拖拽空白区：撑满标题栏剩余空间；按钮是它的兄弟节点（无 drag 祖先），
     保证点按钮不会被 Tauri 拖拽逻辑拦截。 -->
<div class="drag" data-tauri-drag-region></div>

{#if showControls}
  <button
    class="cap"
    title="最小化"
    aria-label="最小化"
    onclick={minimize}
  >
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <line x1="0.5" y1="5" x2="9.5" y2="5" stroke="currentColor" stroke-width="1" />
    </svg>
  </button>
  <button
    class="cap"
    title={maximized ? "向下还原" : "最大化"}
    aria-label={maximized ? "向下还原" : "最大化"}
    onclick={toggleMax}
  >
    {#if maximized}
      <!-- 还原：前后两个错位方框 -->
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1" aria-hidden="true">
        <rect x="0.5" y="2.5" width="7" height="7" />
        <path d="M2.5 0.5 H9.5 V7.5" />
      </svg>
    {:else}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1" aria-hidden="true">
        <rect x="0.5" y="0.5" width="9" height="9" />
      </svg>
    {/if}
  </button>
  <button
    class="cap close"
    title="关闭"
    aria-label="关闭"
    onclick={close}
  >
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <line x1="0.5" y1="0.5" x2="9.5" y2="9.5" stroke="currentColor" stroke-width="1" />
      <line x1="9.5" y1="0.5" x2="0.5" y2="9.5" stroke="currentColor" stroke-width="1" />
    </svg>
  </button>
{/if}

<style>
  /* 拖拽区占满标题栏剩余空间（flex:1）；按钮固定宽度靠右 */
  .drag {
    flex: 1;
    height: 100%;
  }

  .cap {
    flex: 0 0 auto;
    width: 40px;
    height: 100%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    padding: 0;
    background: transparent;
    color: var(--text-primary);
    cursor: default;
    font-family: inherit;
    -webkit-app-region: no-drag;
  }
  .cap:hover {
    background: var(--caption-hover);
  }
  .cap:active {
    background: var(--caption-active);
  }
  /* Windows 关闭键悬停红色（对齐 Win11 caption 习惯）*/
  .cap.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .cap.close:active {
    background: #b0271b;
    color: #fff;
  }
</style>
