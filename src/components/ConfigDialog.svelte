<script lang="ts">
  // API Key 配置对话框：密码遮罩 + 显示切换 + 保存刷新。
  // 对应 legacy widget.py 的 ConfigDialog。
  import { onMount } from "svelte";
  import { getKeys, saveKeys } from "../api";

  let {
    onclose,
  }: { onclose: () => void } = $props();

  let deepseek = $state("");
  let glm = $state("");
  let minimax = $state("");
  let showKeys = $state(false);
  let saving = $state(false);
  let error = $state("");

  onMount(async () => {
    try {
      const keys = await getKeys();
      deepseek = keys.deepseekApiKey;
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
      await saveKeys(deepseek, glm, minimax);
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
    <div class="hint">
      配置各厂商 Coding Plan 的 API Key。留空则清除该厂商的已保存配置。
    </div>

    <label class="field">
      <span class="lbl">DeepSeek</span>
      <input
        type={showKeys ? "text" : "password"}
        placeholder="sk-..."
        bind:value={deepseek}
      />
    </label>

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
    background: #f4f7fb;
    border-radius: 12px;
    padding: 20px 20px 16px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
  }
  .hint {
    color: #586070;
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
    color: #1e2433;
    font-size: 12px;
    font-weight: 600;
    flex-shrink: 0;
  }
  input[type="text"],
  input[type="password"] {
    flex: 1;
    padding: 6px 8px;
    border: 1px solid rgba(0, 0, 0, 0.12);
    border-radius: 6px;
    background: white;
    color: #1e2433;
    font-size: 12px;
    font-family: inherit;
  }
  input:focus {
    outline: none;
    border-color: #4f87ff;
  }
  .show-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #586070;
    font-size: 11px;
    margin-bottom: 12px;
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
    border: 1px solid rgba(0, 0, 0, 0.1);
    background: white;
    color: #1e2433;
  }
  .cancel:hover {
    background: #eef3fb;
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
