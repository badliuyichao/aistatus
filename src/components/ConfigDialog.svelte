<script lang="ts">
  // 设置对话框：主题选择 + 小米 MiMo Cookie（运行期配置）。
  // 主题即时生效（点选即应用 + 保存）；GLM/MiniMax 的 API Key 为编译期
  // 加密硬编码，不由用户配置。MiMo 的用量接口只认浏览器登录 Cookie，
  // 用户从 DevTools 复制后粘贴于此，保存即触发刷新验证。
  import { onDestroy, onMount } from "svelte";
  import { theme } from "../stores/theme.svelte";
  import type { Theme } from "../types";
  import { getMimoCookie, setMimoCookie } from "../api";

  let {
    onclose,
  }: { onclose: () => void } = $props();

  const themeOpts: { id: Theme; label: string }[] = [
    { id: "system", label: "跟随系统" },
    { id: "dark", label: "暗色" },
    { id: "light", label: "浅色" },
  ];

  // ── MiMo Cookie 状态 ──
  // savedCookie：后端当前已保存值（空串 = 未配置）；mimoCookie：编辑框值。
  // 组件由 {#if} 条件渲染，每次打开重新 onMount 回显最新值。
  let savedCookie = $state("");
  let mimoCookie = $state("");
  // "" 空闲 | "saving" 保存中 | "saved" 已保存
  let saveState = $state<"" | "saving" | "saved">("");
  let saveError = $state("");
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  const configured = $derived(savedCookie !== "");
  // 尾部指纹，确认保存的是哪份 Cookie（完整值太长不宜整段展示）
  const cookieTail = $derived(savedCookie.slice(-12));
  // 有实际修改且非空才可保存；清除走独立按钮
  const canSave = $derived(
    saveState !== "saving" && mimoCookie.trim() !== "" && mimoCookie !== savedCookie
  );

  onMount(async () => {
    try {
      savedCookie = await getMimoCookie();
      mimoCookie = savedCookie;
    } catch (e) {
      console.error("get_mimo_cookie failed", e);
    }
  });

  // async onMount 无法返回 cleanup，自动关闭的定时器在此统一清理
  onDestroy(() => clearTimeout(closeTimer));

  // 按 Escape 取消（对应原生对话框行为）
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }

  async function saveCookie() {
    saveState = "saving";
    saveError = "";
    try {
      await setMimoCookie(mimoCookie);
      savedCookie = mimoCookie.trim();
      saveState = "saved";
      // 后端保存后已触发刷新；稍候自动关闭让用户看到卡片更新结果
      closeTimer = setTimeout(() => onclose(), 800);
    } catch (e) {
      saveState = "";
      saveError = String(e);
      console.error("save_mimo_cookie failed", e);
    }
  }

  async function clearCookie() {
    saveState = "saving";
    saveError = "";
    try {
      await setMimoCookie("");
      savedCookie = "";
      mimoCookie = "";
      saveState = "saved";
      closeTimer = setTimeout(() => onclose(), 800);
    } catch (e) {
      saveState = "";
      saveError = String(e);
      console.error("save_mimo_cookie failed", e);
    }
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
      <div class="section-label">
        小米 MiMo Cookie
        <span class="cookie-state" class:ok={configured}>
          {configured ? `已配置 · …${cookieTail}` : "未配置"}
        </span>
      </div>
      <textarea
        class="cookie-input"
        bind:value={mimoCookie}
        rows="3"
        spellcheck="false"
        autocomplete="off"
        placeholder="粘贴 platform.xiaomimimo.com 的登录 Cookie（获取步骤见下方）"
      ></textarea>
      <div class="cookie-actions">
        {#if saveState === "saving"}
          <button class="btn save" disabled>保存中…</button>
        {:else if saveState === "saved"}
          <button class="btn save" disabled>已保存 ✓ 刷新中</button>
        {:else}
          <button class="btn save" disabled={!canSave} onclick={saveCookie}>
            保存并刷新
          </button>
        {/if}
        <button
          class="btn clear"
          disabled={!configured || saveState === "saving"}
          onclick={clearCookie}
        >
          清除
        </button>
      </div>
      {#if saveError}
        <div class="cookie-error">{saveError}</div>
      {/if}
      <details class="cookie-help">
        <summary>如何获取 Cookie</summary>
        <ol>
          <li>浏览器登录 <span class="mono">platform.xiaomimimo.com</span></li>
          <li>打开「控制台 → 套餐管理」页面</li>
          <li>按 F12 打开开发者工具 →「网络」→ 刷新页面</li>
          <li>找到名为 <span class="mono">usage</span> 的请求 →「标头」→ 复制请求标头里 Cookie 的整段值</li>
          <li>粘贴到上方输入框保存（支持直接粘贴 cURL 命令，会自动提取）</li>
        </ol>
        <p>Cookie 过期后卡片会提示失效，回到此处重新粘贴即可。</p>
      </details>
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
    display: flex;
    align-items: center;
    justify-content: space-between;
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

  .cookie-state {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    color: var(--text-muted);
    font-family: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 220px;
    margin-left: 8px;
  }
  .cookie-state.ok {
    color: #16a34a;
  }
  .cookie-input {
    width: 100%;
    box-sizing: border-box;
    padding: 7px 10px;
    border: 1px solid var(--dialog-input-border);
    border-radius: 6px;
    background: var(--dialog-input-bg);
    color: var(--text-primary);
    font-size: 11px;
    font-family: Consolas, Menlo, monospace;
    resize: vertical;
    outline: none;
  }
  .cookie-input:focus {
    border-color: var(--dialog-input-focus);
  }
  .cookie-input::placeholder {
    color: var(--text-muted);
    font-family: inherit;
  }
  .cookie-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .cookie-actions .save {
    flex: 1;
  }
  .cookie-actions .clear {
    padding: 6px 16px;
    border: 1px solid var(--dialog-cancel-border);
    border-radius: 6px;
    background: var(--dialog-cancel-bg);
    color: var(--text-secondary);
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }
  .cookie-actions .clear:hover:not(:disabled) {
    background: var(--dialog-cancel-hover);
  }
  .cookie-actions .clear:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .cookie-actions .save:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .cookie-error {
    margin-top: 6px;
    color: #dc2626;
    font-size: 11px;
  }
  .cookie-help {
    margin-top: 8px;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.6;
    user-select: text;
  }
  .cookie-help summary {
    cursor: pointer;
    color: var(--text-secondary);
  }
  .cookie-help ol {
    margin: 6px 0 4px;
    padding-left: 18px;
  }
  .cookie-help p {
    margin: 4px 0 0;
  }
  .cookie-help .mono {
    font-family: Consolas, Menlo, monospace;
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
  .save {
    border: none;
    background: #2563eb;
    color: white;
    font-weight: 600;
  }
  .save:hover:not(:disabled) {
    background: #1d4ed8;
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
