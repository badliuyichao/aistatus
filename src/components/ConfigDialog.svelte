<script lang="ts">
  // 设置对话框：主题选择 + API Key 配置。
  // 主题：即时生效（点选即应用 + 保存），不等 Key 的"保存并刷新"。
  // Key：保存并刷新（保存后后端立即重新拉取）。
  import { onMount } from "svelte";
  import { getKeys, saveKeys } from "../api";
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

  let glm = $state("");
  let minimax = $state("");
  let showKeys = $state(false);
  let saving = $state(false);
  let error = $state("");

  onMount(async () => {
    try {
      const keys = await getKeys();
      glm = keys.glmApiKey;
      minimax = keys.minimaxApiKey;
    } catch (e) {
      console.error("get_keys failed", e);
    }
  });

  async function save() {
    saving = true;
    error = "";
    try {
      await saveKeys(glm, minimax);
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

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

    <div class="section">
      <div class="section-label">API Key</div>
      <div class="hint">
        留空则清除该厂商的已保存配置。保存后自动刷新数据。
      </div>

      <label class="field">
        <span class="lbl">GLM 智谱AI</span>
        <input
          type={showKeys ? "text" : "password"}
          placeholder="xxxxxxxx.xxxxxxxxxxxxxxxx"
          bind:value={glm}
        />
      </label>

      <label class="field">
        <span class="lbl">MiniMax</span>
        <input
          type={showKeys ? "text" : "password"}
          placeholder="sk-cp-..."
          bind:value={minimax}
        />
      </label>

      <label class="show-toggle">
        <input type="checkbox" bind:checked={showKeys} />
        <span>显示 Key</span>
      </label>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <div class="actions">
      <button class="btn cancel" onclick={onclose}>取消</button>
      <button class="btn save" onclick={save} disabled={saving}>
        {saving ? "保存中…" : "保存并刷新"}
      </button>
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
  .hint {
    color: var(--text-secondary);
    font-size: 11px;
    margin-bottom: 12px;
    line-height: 1.5;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }
  .lbl {
    width: 90px;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    flex-shrink: 0;
  }
  input[type="text"],
  input[type="password"] {
    flex: 1;
    padding: 6px 8px;
    border: 1px solid var(--dialog-input-border);
    border-radius: 6px;
    background: var(--dialog-input-bg);
    color: var(--text-primary);
    font-size: 12px;
    font-family: inherit;
  }
  input:focus {
    outline: none;
    border-color: var(--dialog-input-focus);
  }
  .show-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: 11px;
  }
  .error {
    color: #ff4444;
    font-size: 11px;
    margin-bottom: 10px;
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
  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .cancel {
    border: 1px solid var(--dialog-cancel-border);
    background: var(--dialog-cancel-bg);
    color: var(--text-primary);
  }
  .cancel:hover {
    background: var(--dialog-cancel-hover);
  }
  .save {
    border: none;
    background: #2563eb;
    color: white;
    font-weight: 600;
  }
  .save:hover {
    background: #1d4ed8;
  }
</style>
