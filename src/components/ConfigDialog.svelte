<script lang="ts">
  // 设置对话框：仅主题选择。
  // 主题即时生效（点选即应用 + 保存）。API Key 已改为编译期加密硬编码，
  // 不再由用户配置，故本对话框不再有 key 输入区。
  import { theme } from "../stores/theme.svelte";
  import type { Theme } from "../types";

  let {
    onclose,
  }: { onclose: () => void } = $props();

  const themeOpts: { id: Theme; label: string }[] = [
    { id: "system", label: "跟随系统" },
    { id: "dark", label: "暗色" },
    { id: "light", label: "浅色" },
  ];

  // 按 Escape 取消（对应原生对话框行为）
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div
  class="overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onclose();
  }}
>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title">
    <div class="dlg-title" id="dlg-title">设置</div>

    <div class="section">
      <div class="section-label">主题</div>
      <div class="theme-options">
        {#each themeOpts as opt (opt.id)}
          <button
            type="button"
            class="theme-opt"
            class:active={theme.pref === opt.id}
            onclick={() => theme.set(opt.id)}
          >
            {opt.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="actions">
      <button class="btn done" onclick={onclose}>完成</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    user-select: none;
  }
  .dialog {
    width: 460px;
    max-width: 90%;
    background: var(--dialog-bg);
    border-radius: 12px;
    padding: 20px 20px 16px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  }
  .dlg-title {
    color: var(--text-primary);
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 14px;
  }
  .section {
    margin-bottom: 16px;
  }
  .section-label {
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 600;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .theme-options {
    display: flex;
    gap: 6px;
  }
  .theme-opt {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid var(--dialog-cancel-border);
    border-radius: 6px;
    background: var(--dialog-cancel-bg);
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }
  .theme-opt:hover {
    background: var(--dialog-cancel-hover);
  }
  .theme-opt.active {
    border-color: #2563eb;
    background: #2563eb;
    color: #fff;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    padding: 6px 16px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
  }
  .done {
    border: none;
    background: #2563eb;
    color: white;
    font-weight: 600;
  }
  .done:hover {
    background: #1d4ed8;
  }
</style>
