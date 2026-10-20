<script lang="ts">
  // 设置对话框：主题选择 + Coding Plan 实例管理 + 小米 MiMo Cookie。
  // 主题即时生效（点选即应用 + 保存）。Coding Plan 实例支持同品牌多份
  // （如 2 个 GLM coding plan、2 个 DeepSeek key）：添加 / 备注 / 换 Key /
  // 显隐 / 删除，整体乐观更新、失败回滚，保存后端即时重发快照并触发刷新。
  // MiMo 的用量接口只认浏览器登录 Cookie，从 DevTools 复制后粘贴于下方区块
  // （存到 plans 里的 mimo 实例），保存即触发刷新验证。
  import { onDestroy, onMount } from "svelte";
  import { theme } from "../stores/theme.svelte";
  import type { PlanConfig, Theme } from "../types";
  import { getMimoCookie, setMimoCookie, getPlans, savePlans } from "../api";

  let {
    onclose,
  }: { onclose: () => void } = $props();

  const themeOpts: { id: Theme; label: string }[] = [
    { id: "system", label: "跟随系统" },
    { id: "dark", label: "暗色" },
    { id: "light", label: "浅色" },
  ];

  // ── Coding Plan 实例管理 ──
  // id 为实例稳定 ID（后端生成），不随显示名变化；显隐 / 删除 / 换 Key 都按 id。
  const vendorNames: Record<string, string> = {
    glm: "GLM 智谱AI",
    minimax: "MiniMax",
    mimo: "小米 MiMo",
    deepseek: "DeepSeek",
  };
  // 添加表单可选的服务商；MiMo 走下方 Cookie 区块，不在此添加。
  const addableVendors: { id: string; label: string }[] = [
    { id: "glm", label: "GLM" },
    { id: "minimax", label: "MiniMax" },
    { id: "deepseek", label: "DeepSeek" },
  ];

  let plans = $state<PlanConfig[]>([]);
  let plansError = $state("");
  let plansBusy = $state(false); // 保存中，防连点

  // 添加表单状态
  let addVendor = $state("glm");
  let addLabel = $state("");
  let addKey = $state("");

  // 行内换 Key：正在编辑的实例 id + 输入值（不回显旧 key，避免误改）
  let editKeyId = $state("");
  let editKeyValue = $state("");

  // 至少保留一个实例；至少一个可见
  const canDelete = $derived(plans.length > 1);
  const visibleCount = $derived(plans.filter((p) => p.visible).length);

  /** 凭据展示：内置实例显示「内置」（key 编译在二进制里，不落盘不可见），
   *  普通实例显示尾部指纹，确认对应哪个 key / cookie（完整值不宜展示）。 */
  function credTail(p: PlanConfig): string {
    if (p.builtin != null) return "内置";
    const cred = p.vendor === "mimo" ? p.cookie : p.key;
    return cred ? `…${cred.slice(-8)}` : "未配置";
  }

  /** 乐观更新 + 失败回滚。成功后以后端规范化结果为准
   *  （新增实例的 id 由后端生成）。mutate 返回 null 表示校验不过，不动。 */
  async function persist(mutate: (list: PlanConfig[]) => PlanConfig[] | null) {
    if (plansBusy) return;
    const next = mutate(plans);
    if (next === null) return;
    const prev = plans;
    plans = next;
    plansBusy = true;
    plansError = "";
    try {
      plans = await savePlans(next);
    } catch (e) {
      plans = prev;
      plansError = String(e);
      console.error("save_plans failed", e);
    } finally {
      plansBusy = false;
    }
  }

  function toggleVisible(p: PlanConfig, checked: boolean) {
    persist((list) => {
      if (!checked && list.filter((x) => x.visible).length <= 1) return null;
      return list.map((x) => (x.id === p.id ? { ...x, visible: checked } : x));
    });
  }

  function removePlan(p: PlanConfig) {
    persist((list) => (list.length <= 1 ? null : list.filter((x) => x.id !== p.id)));
  }

  /** 上移 / 下移实例：显示顺序 = plans 数组顺序（主窗口卡片与悬浮条一致）。 */
  function movePlan(p: PlanConfig, dir: -1 | 1) {
    const idx = plans.findIndex((x) => x.id === p.id);
    const next = idx + dir;
    if (idx < 0 || next < 0 || next >= plans.length || plansBusy) return;
    const list = [...plans];
    [list[idx], list[next]] = [list[next], list[idx]];
    persist(() => list);
  }

  function renamePlan(p: PlanConfig, label: string) {
    const v = label.trim();
    if (v === p.label) return;
    const target = p.id;
    persist((list) => list.map((x) => (x.id === target ? { ...x, label: v } : x)));
  }

  function toggleEditKey(p: PlanConfig) {
    if (editKeyId === p.id) {
      cancelEditKey();
    } else {
      editKeyId = p.id;
      editKeyValue = "";
    }
  }

  function confirmEditKey(p: PlanConfig) {
    const v = editKeyValue.trim();
    if (!v) return;
    const target = p.id;
    editKeyId = "";
    editKeyValue = "";
    // 换 Key 会断开内置引用（builtin 清空），改为使用粘贴的明文 Key
    persist((list) =>
      list.map((x) => (x.id === target ? { ...x, key: v, builtin: null } : x))
    );
  }

  function cancelEditKey() {
    editKeyId = "";
    editKeyValue = "";
  }

  function addPlan() {
    const key = addKey.trim();
    if (!key || plansBusy) return;
    const entry: PlanConfig = {
      id: "", // 由后端 normalize 生成
      vendor: addVendor,
      label: addLabel.trim(),
      key,
      cookie: "",
      visible: true,
    };
    addKey = "";
    addLabel = "";
    persist((list) => [...list, entry]);
  }

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

  /** Cookie 存的是 plans 里的 mimo 实例，保存后同步实例列表（尾部指纹等）。 */
  async function syncPlans() {
    try {
      plans = await getPlans();
    } catch (e) {
      console.error("get_plans failed", e);
    }
  }

  onMount(async () => {
    await syncPlans();
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
      await syncPlans();
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
      await syncPlans();
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
        Coding Plan 管理
        <span class="plan-count">{plans.length} 个实例 · {visibleCount} 个可见</span>
      </div>
      <div class="plan-list">
        {#each plans as p, idx (p.id)}
          <div class="plan-row">
            <label class="plan-vis" title={p.visible ? "点击隐藏" : "点击显示"}>
              <input
                type="checkbox"
                checked={p.visible}
                disabled={plansBusy || (p.visible && visibleCount <= 1)}
                onchange={(e) => toggleVisible(p, e.currentTarget.checked)}
              />
            </label>
            <span class="plan-vendor">{vendorNames[p.vendor] ?? p.vendor}</span>
            <input
              class="plan-label"
              value={p.label}
              placeholder="备注名"
              title="备注名（区分同品牌实例）"
              disabled={plansBusy}
              onchange={(e) => renamePlan(p, e.currentTarget.value)}
              onkeydown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
              }}
            />
            <span
              class="plan-tail"
              title={p.builtin != null
                ? "Key 编译内置（不落盘不可见）"
                : p.vendor === "mimo"
                  ? "Cookie"
                  : "API Key"}
            >
              {p.vendor === "mimo" ? "Cookie " : ""}{credTail(p)}
            </span>
            {#if p.vendor === "mimo"}
              <span class="plan-hint">Cookie 在下方</span>
            {:else}
              <button
                class="mini-btn"
                disabled={plansBusy}
                onclick={() => toggleEditKey(p)}
                title={p.builtin != null
                  ? "更换后将断开内置引用，改用粘贴的 Key"
                  : "更换该实例的 API Key"}
              >
                换 Key
              </button>
            {/if}
            <button
              class="mini-btn move"
              disabled={plansBusy || idx === 0}
              onclick={() => movePlan(p, -1)}
              title="上移（列表顺序 = 卡片与悬浮条顺序）"
            >
              ↑
            </button>
            <button
              class="mini-btn move"
              disabled={plansBusy || idx === plans.length - 1}
              onclick={() => movePlan(p, 1)}
              title="下移（列表顺序 = 卡片与悬浮条顺序）"
            >
              ↓
            </button>
            <button
              class="mini-btn danger"
              disabled={plansBusy || !canDelete}
              onclick={() => removePlan(p)}
              title={canDelete ? "删除该实例" : "至少保留一个实例"}
            >
              删
            </button>
          </div>
          {#if editKeyId === p.id}
            <div class="plan-key-row">
              <input
                class="plan-key-input"
                placeholder="粘贴新的 API Key"
                bind:value={editKeyValue}
                spellcheck="false"
                autocomplete="off"
              />
              <button
                class="mini-btn ok"
                disabled={!editKeyValue.trim() || plansBusy}
                onclick={() => confirmEditKey(p)}
              >
                保存
              </button>
              <button class="mini-btn" onclick={cancelEditKey}>取消</button>
            </div>
          {/if}
        {/each}
      </div>

      <details class="plan-add">
        <summary>添加实例（同品牌可添加多个）</summary>
        <div class="add-vendors">
          {#each addableVendors as v (v.id)}
            <button
              type="button"
              class="vendor-opt"
              class:active={addVendor === v.id}
              onclick={() => (addVendor = v.id)}
            >
              {v.label}
            </button>
          {/each}
        </div>
        <input
          class="add-label"
          placeholder="备注名（可选，如 主号 / 备用）"
          bind:value={addLabel}
        />
        <textarea
          class="add-key"
          rows="2"
          placeholder="粘贴该账号的 API Key"
          bind:value={addKey}
          spellcheck="false"
          autocomplete="off"
        ></textarea>
        <div class="add-actions">
          <button class="btn save" disabled={!addKey.trim() || plansBusy} onclick={addPlan}>
            添加并刷新
          </button>
        </div>
      </details>

      {#if plansError}
        <div class="cookie-error">{plansError}</div>
      {/if}
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

  /* ── Coding Plan 实例列表 ── */
  .plan-count {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    color: var(--text-muted);
  }
  .plan-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .plan-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 2px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-primary);
  }
  .plan-row:hover {
    background: var(--dialog-cancel-hover);
  }
  .plan-vis {
    display: flex;
    align-items: center;
    margin: 0;
  }
  .plan-vis input {
    accent-color: #2563eb;
    cursor: pointer;
    margin: 0;
  }
  .plan-vis input:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .plan-vendor {
    flex: none;
    width: 72px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .plan-label {
    flex: 1;
    min-width: 0;
    padding: 2px 6px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    color: var(--text-primary);
    font-size: 11px;
    font-family: inherit;
    outline: none;
  }
  .plan-label::placeholder {
    color: var(--text-muted);
  }
  .plan-label:hover {
    border-color: var(--dialog-cancel-border);
  }
  .plan-label:focus {
    border-color: var(--dialog-input-focus);
    background: var(--dialog-input-bg);
  }
  .plan-tail {
    flex: none;
    max-width: 88px;
    color: var(--text-muted);
    font-size: 10px;
    font-family: Consolas, Menlo, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .plan-hint {
    flex: none;
    color: var(--text-muted);
    font-size: 10px;
  }
  .mini-btn {
    flex: none;
    padding: 2px 7px;
    border: 1px solid var(--dialog-cancel-border);
    border-radius: 4px;
    background: var(--dialog-cancel-bg);
    color: var(--text-secondary);
    font-size: 10px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }
  .mini-btn:hover:not(:disabled) {
    background: var(--dialog-cancel-hover);
  }
  .mini-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .mini-btn.ok {
    color: #16a34a;
    border-color: #16a34a;
  }
  /* ↑↓ 调序按钮：更紧凑，给整行留空间 */
  .mini-btn.move {
    padding: 2px 5px;
  }
  .mini-btn.danger {
    color: #dc2626;
  }
  /* 行内换 Key 输入行 */
  .plan-key-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 2px 5px 24px;
  }
  .plan-key-input {
    flex: 1;
    min-width: 0;
    padding: 3px 8px;
    border: 1px solid var(--dialog-input-border);
    border-radius: 4px;
    background: var(--dialog-input-bg);
    color: var(--text-primary);
    font-size: 11px;
    font-family: Consolas, Menlo, monospace;
    outline: none;
  }
  .plan-key-input:focus {
    border-color: var(--dialog-input-focus);
  }

  /* 添加实例表单（可折叠） */
  .plan-add {
    margin-top: 6px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .plan-add summary {
    cursor: pointer;
    color: var(--text-secondary);
    user-select: none;
  }
  .add-vendors {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .vendor-opt {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--dialog-cancel-border);
    border-radius: 6px;
    background: var(--dialog-cancel-bg);
    color: var(--text-primary);
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
  }
  .vendor-opt.active {
    border-color: #2563eb;
    background: #2563eb;
    color: #fff;
  }
  .add-label {
    width: 100%;
    box-sizing: border-box;
    margin-top: 6px;
    padding: 5px 8px;
    border: 1px solid var(--dialog-input-border);
    border-radius: 6px;
    background: var(--dialog-input-bg);
    color: var(--text-primary);
    font-size: 11px;
    font-family: inherit;
    outline: none;
  }
  .add-label:focus {
    border-color: var(--dialog-input-focus);
  }
  .add-key {
    width: 100%;
    box-sizing: border-box;
    margin-top: 6px;
    padding: 5px 8px;
    border: 1px solid var(--dialog-input-border);
    border-radius: 6px;
    background: var(--dialog-input-bg);
    color: var(--text-primary);
    font-size: 11px;
    font-family: Consolas, Menlo, monospace;
    resize: vertical;
    outline: none;
  }
  .add-key:focus {
    border-color: var(--dialog-input-focus);
  }
  .add-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 6px;
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
  .save:disabled {
    opacity: 0.6;
    cursor: default;
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
