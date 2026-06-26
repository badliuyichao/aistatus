<script lang="ts">
  // 自定义拖拽标题栏 + 窗口控制按钮。
  // 对应 legacy widget.py 的 TitleBar。
  // 拖拽用 Tauri 的 data-tauri-drag-region；窗口控制用 @tauri-apps/api/window。
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let {
    title,
    onconfig,
  }: { title: string; onconfig: () => void } = $props();

  const appWindow = getCurrentWindow();
  let pinned = $state(true);

  async function togglePin() {
    pinned = !pinned;
    await appWindow.setAlwaysOnTop(pinned);
  }
  function minimize() {
    appWindow.minimize();
  }
  function close() {
    appWindow.hide(); // 对应 legacy _on_close：隐藏到托盘而非退出
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>{title}</span>
  <div class="spacer" data-tauri-drag-region></div>
  <button class="btn config" title="配置 API Key" onclick={onconfig}>⚙</button>
  <button class="btn pin" class:active={pinned} title={pinned ? "取消置顶" : "置顶"} onclick={togglePin}>📌</button>
  <button class="btn" title="最小化" onclick={minimize}>─</button>
  <button class="btn close" title="关闭" onclick={close}>✕</button>
</div>

<style>
  .titlebar {
    height: 36px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 4px 0 12px;
  }
  .title {
    color: #1e2433;
    font-size: 13px;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .btn {
    width: 28px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: #8a92a0;
    font-size: 13px;
    font-weight: bold;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .btn:hover {
    background: rgba(0, 0, 0, 0.06);
    color: #1e2433;
  }
  .config {
    font-size: 14px;
  }
  .pin.active {
    color: #fff;
    background: #2563eb;
    font-size: 12px;
  }
  .pin.active:hover {
    background: #1d4ed8;
    color: #fff;
  }
  .pin:not(.active) {
    font-size: 12px;
    color: #9aa1ad;
  }
  .close:hover {
    background: rgba(255, 60, 60, 0.3);
    color: #ff4444;
  }
</style>
