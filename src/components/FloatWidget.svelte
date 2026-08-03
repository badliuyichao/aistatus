<script lang="ts">
  // 悬浮条：长方形小窗，两条进度条分别显示 GLM / MiniMax 的「5小时限额」已用%。
  //
  // 与主窗口共用同一份 index.html，经 main.ts 按 webview label 分流挂载。
  // 复用 services store（自动收 services-updated 广播）和 theme store（跟随主题）。
  //
  // 数据来源：只取每家 items 里 label === "5小时限额" 的配额项。
  //   - item.used 已是 0–100 的「已用百分比」，total=100，故进度条宽度直接用 used。
  //   - item.displayValue === "∞" 表示该窗口未计费/不限额，进度条满格灰色 + ∞ 标记。
  //   - 服务 loading 或找不到 5小时限额 item 时显示 N/A。

  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { services } from "../stores/services.svelte";
  import { theme } from "../stores/theme.svelte";
  import { saveFloatPosition } from "../api";
  import type { QuotaItem, ServiceInfo } from "../types";

  const appWindow = getCurrentWindow();
  let moveTimer: number | undefined;
  let unlistenMoved: (() => void) | undefined;

  /** 匹配规则与后端 merge.rs 对齐：GLM 名字以 "GLM 智谱AI" 开头，MiniMax 名字为 "MiniMax"。 */
  function findService(prefix: string): ServiceInfo | undefined {
    return services.data.services.find(
      (s) => s.name.startsWith(prefix) && s.type === "quota"
    );
  }

  /** 取某家「5小时限额」item；服务 loading 或无此 item 返回 undefined。 */
  function fiveHour(svc?: ServiceInfo): QuotaItem | undefined {
    if (!svc || svc.loading) return undefined;
    return svc.items.find((it) => it.label === "5小时限额");
  }

  /** 已用百分比（0–100）。∞ / 无数据返回 null。 */
  function usedPct(item?: QuotaItem): number | null {
    if (!item) return null;
    if (item.displayValue === "∞") return null;
    // item.used 已是百分比，clamp 到 [0,100] 防越界。
    return Math.max(0, Math.min(100, item.used));
  }

  /** 进度条填充色按已用%分级：绿<50 / 黄 50–80 / 红>80。 */
  function barColor(pct: number | null, svc?: ServiceInfo): string {
    if (pct === null) return "var(--bar-track)"; // ∞ / N/A：灰色占位
    if (pct >= 80) return "#ef4444"; // 红：将耗尽
    if (pct >= 50) return "#f59e0b"; // 黄：偏高
    return svc?.color || "#22c55e"; // 绿：充足，用品牌色
  }

  const glm = $derived(findService("GLM"));
  const mm = $derived(findService("MiniMax"));
  const glmItem = $derived(fiveHour(glm));
  const mmItem = $derived(fiveHour(mm));
  const glmPct = $derived(usedPct(glmItem));
  const mmPct = $derived(usedPct(mmItem));

  /** 启动拖拽（Tauri 原生窗口拖动）。 */
  function startDrag(e: MouseEvent) {
    e.preventDefault();
    appWindow.startDragging();
  }

  onMount(() => {
    (async () => {
      await theme.init();
      await services.init();
      unlistenMoved = await appWindow.onMoved(() => {
        if (moveTimer !== undefined) clearTimeout(moveTimer);
        moveTimer = window.setTimeout(async () => {
          try {
            const factor = await appWindow.scaleFactor();
            const pos = await appWindow.outerPosition();
            await saveFloatPosition(pos.x / factor, pos.y / factor);
          } catch (e) {
            console.error("save float position failed", e);
          }
        }, 200);
      });
    })();

    return () => {
      if (moveTimer !== undefined) clearTimeout(moveTimer);
      unlistenMoved?.();
    };
  });
</script>

<div
  class="floatbar"
  onmousedown={startDrag}
  role="button"
  tabindex="-1"
  title="拖拽移动 · 托盘菜单「悬浮条」切换显隐"
>
  <div class="row">
    <span class="brand" style="color:{glm?.color || '#888'}">GLM</span>
    <div class="track">
      {#if glmPct === null && glmItem?.displayValue === "∞"}
        <div class="fill infinite" style="width:100%"></div>
      {:else if glmPct !== null}
        <div class="fill" style="width:{glmPct}%; background:{barColor(glmPct, glm)}"></div>
      {/if}
    </div>
    <span class="pct">
      {#if glmPct === null}
        {glmItem?.displayValue ?? "—"}
      {:else}
        {Math.round(glmPct)}%
      {/if}
    </span>
  </div>

  <div class="row">
    <span class="brand" style="color:{mm?.color || '#888'}">MiniMax</span>
    <div class="track">
      {#if mmPct === null && mmItem?.displayValue === "∞"}
        <div class="fill infinite" style="width:100%"></div>
      {:else if mmPct !== null}
        <div class="fill" style="width:{mmPct}%; background:{barColor(mmPct, mm)}"></div>
      {/if}
    </div>
    <span class="pct">
      {#if mmPct === null}
        {mmItem?.displayValue ?? "—"}
      {:else}
        {Math.round(mmPct)}%
      {/if}
    </span>
  </div>
</div>

<style>
  /* 220×96 webview；body 已透明（见 app.css），卡片自身做半透明质感。 */
  .floatbar {
    width: 100vw;
    height: 100vh;
    padding: 8px 10px;
    border-radius: 10px;
    /* 半透明卡片底，桌面深浅都能看清。 */
    background: var(--card-bg, rgba(28, 32, 44, 0.82));
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    cursor: grab;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.28);
    color: var(--text-primary, #1e2433);
    user-select: none;
  }
  .floatbar:active {
    cursor: grabbing;
  }

  .row {
    display: grid;
    grid-template-columns: 46px 1fr 30px;
    align-items: center;
    gap: 6px;
  }

  .brand {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-primary, #1e2433);
  }

  .track {
    height: 6px;
    border-radius: 3px;
    background: var(--bar-track, rgba(255, 255, 255, 0.12));
    overflow: hidden;
    position: relative;
  }

  .fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.4s ease, background 0.4s ease;
  }
  /* ∞ 不限额：满格但用中性色 + 斜纹，与「100% 已用」的红区分。 */
  .fill.infinite {
    background: repeating-linear-gradient(
      45deg,
      rgba(180, 190, 210, 0.5),
      rgba(180, 190, 210, 0.5) 4px,
      rgba(140, 150, 170, 0.5) 4px,
      rgba(140, 150, 170, 0.5) 8px
    );
  }

  .pct {
    font-size: 10px;
    font-weight: 600;
    text-align: right;
    color: var(--text-primary, #1e2433);
    font-variant-numeric: tabular-nums;
  }
</style>
