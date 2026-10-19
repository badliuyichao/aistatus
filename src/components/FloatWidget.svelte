<script lang="ts">
  // 悬浮条：长方形小窗，进度条显示 GLM / MiniMax 的「5小时限额」+「周限额」
  // （同一行上下两条）、MiMo 的「Token Plan 套餐」已用%；
  // DeepSeek 为余额型，显示账户余额金额。
  //
  // 与主窗口共用同一份 index.html，经 main.ts 按 webview label 分流挂载。
  // 复用 services store（自动收 services-updated 广播）和 theme store（跟随主题）。
  //
  // 数据来源：GLM / MiniMax 取 items 里 label === "5小时限额" / "周限额" 两条配额项，
  // MiMo 取 "Token Plan 套餐"，DeepSeek 取 type === "balance" 的卡片。
  //   - 百分比统一按 used/total 计算：GLM/MiniMax 的百分比条 total=100；
  //     GLM 若返回 token 绝对值条（unit=="tokens"）也能得到正确比例。
  //   - MiMo：item.used / total 是**原始额度**（如 893M / 4.1B，非百分比），
  //     比例需自行计算 used/total；明细（绝对值 + 已用%）放行 tooltip。
  //   - item.displayValue === "∞" 表示该窗口未计费/不限额，进度条满格灰色 + ∞ 标记。
  //   - 进度条填充用同色相「浅→深」渐变（barGradient）：渐变按整条 track 铺开，
  //     宽度越大露出的颜色越深（5% 只有浅色，100% 露出完整深色端）；
  //     基础色仍按已用%分级（绿<50 / 黄 50–80 / 红>80）。
  //   - DeepSeek：纯 prepaid 按量计费，没有「已用百分比」的语义（无分母可算），
  //     进度条无意义 → 该行不放进度条，右对齐显示余额金额；
  //     金额旁显示状态标签（安全 ≥50 绿 / 预警 10–<50 黄 / 危险 <10 红），
  //     金额与标签同色；错误占位（key 无效 / 服务繁忙）不显示状态。
  //     明细（充值/赠送拆分、错误提示）放行 tooltip。
  //   - 服务 loading 或找不到对应 item 时显示 N/A；被设置对话框隐藏的服务
  //     不会出现在数据里（后端按 visiblePlans 过滤），整行不渲染。
  //
  // GLM / MiniMax 的两条进度条叠在同一行内（各 4px 高，上=5小时、下=本周），
  // 不显示右侧百分比文字，具体数值放行 tooltip；可见行数不变，窗口高度仍由后端
  // fit_float_window 按行数自适应（40/60/80/100 逻辑像素）。

  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { services } from "../stores/services.svelte";
  import { theme } from "../stores/theme.svelte";
  import { saveFloatPosition } from "../api";
  import { balanceStatus } from "../balanceStatus";
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

  /** 取某家指定 label 的配额项；服务 loading 或无此 item 返回 undefined。 */
  function quotaItem(svc: ServiceInfo | undefined, label: string): QuotaItem | undefined {
    if (!svc || svc.loading) return undefined;
    return svc.items.find((it) => it.label === label);
  }

  /** 已用百分比（0–100）。∞ / 无数据返回 null；兼容百分比条与 token 绝对值条。 */
  function usedPct(item?: QuotaItem): number | null {
    if (!item) return null;
    if (item.displayValue === "∞") return null;
    // 百分比条的 total=100，token 条的 total 是原始额度，统一按 used/total 换算。
    const raw = item.total > 0 ? (item.used / item.total) * 100 : item.used;
    return Math.max(0, Math.min(100, raw));
  }

  /** 右侧显示值：有百分比用百分比，∞ / 占位用后端 displayValue，无数据用 —。 */
  function pctLabel(item: QuotaItem | undefined, pct: number | null): string {
    if (pct !== null) return `${Math.round(pct)}%`;
    return item?.displayValue ?? "—";
  }

  /** 进度条基础色按已用%分级：绿<50 / 黄 50–80 / 红>80。 */
  function barColor(pct: number | null, svc?: ServiceInfo): string {
    if (pct === null) return "var(--bar-track)"; // ∞ / N/A：灰色占位
    if (pct >= 80) return "#ef4444"; // 红：将耗尽
    if (pct >= 50) return "#f59e0b"; // 黄：偏高
    return svc?.color || "#22c55e"; // 绿：充足，用品牌色
  }

  /** 把 #rgb / #rrggbb 朝 target 色混合，ratio 是 target 的占比（0–1）。 */
  function mixColor(hex: string, target: string, ratio: number): string {
    const parse = (h: string): [number, number, number] | null => {
      const m = h.trim().match(/^#?([0-9a-f]{3}|[0-9a-f]{6})$/i);
      if (!m) return null;
      let s = m[1];
      if (s.length === 3) s = s.split("").map((c) => c + c).join("");
      return [
        parseInt(s.slice(0, 2), 16),
        parseInt(s.slice(2, 4), 16),
        parseInt(s.slice(4, 6), 16),
      ];
    };
    const a = parse(hex);
    const b = parse(target);
    if (!a || !b) return hex;
    return (
      "#" +
      a
        .map((v, i) => Math.round(v + (b[i] - v) * ratio).toString(16).padStart(2, "0"))
        .join("")
    );
  }

  /**
   * 同色相浅→深渐变：起点接近白色，终点在基础色上压暗。
   * 渐变按整条 track 宽度铺开（见 gradientSize），填充宽度就是已用比例，
   * 所以 5% 只露出最浅的一段，100% 才露出完整的浅→深。
   */
  function barGradient(pct: number, svc?: ServiceInfo): string {
    const base = barColor(pct, svc);
    return `linear-gradient(90deg, ${mixColor(base, "#ffffff", 0.72)}, ${mixColor(base, "#000000", 0.22)})`;
  }

  /** 让渐变图按整条 track 的宽度铺开，而不是按当前填充宽度压缩。 */
  function gradientSize(pct: number): string {
    const size = pct > 0 ? 10000 / pct : 100;
    return `${Math.round(size * 100) / 100}% 100%`;
  }

  const glm = $derived(findService("GLM"));
  const mm = $derived(findService("MiniMax"));
  const mimo = $derived(findService("MiMo"));
  const glmItem = $derived(quotaItem(glm, "5小时限额"));
  const glmWeekItem = $derived(quotaItem(glm, "周限额"));
  const mmItem = $derived(quotaItem(mm, "5小时限额"));
  const mmWeekItem = $derived(quotaItem(mm, "周限额"));
  const glmPct = $derived(usedPct(glmItem));
  const glmWeekPct = $derived(usedPct(glmWeekItem));
  const mmPct = $derived(usedPct(mmItem));
  const mmWeekPct = $derived(usedPct(mmWeekItem));
  // MiMo 的 Token Plan 套餐项；loading 时 undefined（显示 —）。
  const mimoItem = $derived(
    mimo && !mimo.loading
      ? mimo.items.find((it) => it.label === "Token Plan 套餐")
      : undefined
  );
  // MiMo 的 used/total 是原始额度（非百分比），比例自行计算；total<=0 视为无数据。
  const mimoPct = $derived(
    mimoItem && mimoItem.total > 0
      ? Math.max(0, Math.min(100, (mimoItem.used / mimoItem.total) * 100))
      : null
  );

  // DeepSeek 余额卡片（balance 型；后端 merge 保证 items 恒空、金额在 balance 字段）。
  const ds = $derived(
    services.data.services.find(
      (s) => s.name.startsWith("DeepSeek") && s.type === "balance"
    )
  );

  // DeepSeek 余额状态：loading / 错误占位时不显示，避免把 0 元占位误判为危险。
  const dsStatus = $derived(
    ds && !ds.loading && !ds.placeholder ? balanceStatus(ds.balance) : null
  );

  /** 金额格式化：千分位 + 固定两位小数（与主窗口 BalanceView 一致）。 */
  function fmtAmount(v: number): string {
    return v.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  }

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
  oncontextmenu={(e) => e.preventDefault()}
  role="button"
  tabindex="-1"
  title="拖拽移动 · 托盘菜单「悬浮条」切换显隐"
>
  {#if glm}
    <div
      class="row dual-row"
      title="GLM · 5小时限额 {pctLabel(glmItem, glmPct)} · 本周 {pctLabel(glmWeekItem, glmWeekPct)}"
    >
      <span class="brand" style="color:{glm.color || '#888'}">GLM</span>
      <div class="tracks">
        <div class="track thin">
          {#if glmPct === null && glmItem?.displayValue === "∞"}
            <div class="fill infinite" style="width:100%"></div>
          {:else if glmPct !== null}
            <div
              class="fill"
              style="width:{glmPct}%; background-image:{barGradient(glmPct, glm)}; background-size:{gradientSize(glmPct)}"
            ></div>
          {/if}
        </div>
        <div class="track thin">
          {#if glmWeekPct === null && glmWeekItem?.displayValue === "∞"}
            <div class="fill infinite" style="width:100%"></div>
          {:else if glmWeekPct !== null}
            <div
              class="fill"
              style="width:{glmWeekPct}%; background-image:{barGradient(glmWeekPct, glm)}; background-size:{gradientSize(glmWeekPct)}"
            ></div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#if mm}
    <div
      class="row dual-row"
      title="MiniMax · 5小时限额 {pctLabel(mmItem, mmPct)} · 本周 {pctLabel(mmWeekItem, mmWeekPct)}"
    >
      <span class="brand" style="color:{mm.color || '#888'}">MiniMax</span>
      <div class="tracks">
        <div class="track thin">
          {#if mmPct === null && mmItem?.displayValue === "∞"}
            <div class="fill infinite" style="width:100%"></div>
          {:else if mmPct !== null}
            <div
              class="fill"
              style="width:{mmPct}%; background-image:{barGradient(mmPct, mm)}; background-size:{gradientSize(mmPct)}"
            ></div>
          {/if}
        </div>
        <div class="track thin">
          {#if mmWeekPct === null && mmWeekItem?.displayValue === "∞"}
            <div class="fill infinite" style="width:100%"></div>
          {:else if mmWeekPct !== null}
            <div
              class="fill"
              style="width:{mmWeekPct}%; background-image:{barGradient(mmWeekPct, mm)}; background-size:{gradientSize(mmWeekPct)}"
            ></div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  {#if mimo}
    <div
      class="row"
      title="MiMo Token Plan：{mimoItem?.displayValue ?? '—'} · {mimoItem?.detail ?? '刷新中'}"
    >
      <span class="brand" style="color:{mimo.color || '#FF6900'}">MiMo</span>
      <div class="track">
        {#if mimoPct !== null}
          <div
            class="fill"
            style="width:{mimoPct}%; background-image:{barGradient(mimoPct, mimo)}; background-size:{gradientSize(mimoPct)}"
          ></div>
        {/if}
      </div>
      <span class="pct">{mimoPct === null ? "—" : `${Math.round(mimoPct)}%`}</span>
    </div>
  {/if}

  {#if ds}
    <div
      class="row balance-row"
      title="DeepSeek 余额：{ds.loading ? '刷新中' : `${ds.currency}${fmtAmount(ds.balance)}`}{dsStatus ? ` · ${dsStatus.label}` : ''}{ds.detail ? ` · ${ds.detail}` : ''}"
    >
      <span class="brand" style="color:{ds.color || '#4D6BFE'}">DeepSeek</span>
      <div class="balance-right">
        <span
          class="amount"
          style="color:{ds.loading ? 'var(--text-muted)' : dsStatus?.color ?? 'var(--text-muted)'}"
        >
          {ds.loading ? "—" : `${ds.currency}${fmtAmount(ds.balance)}`}
        </span>
        {#if dsStatus}
          <span class="status-badge" style="color:{dsStatus.color}; border-color:{dsStatus.color}">
            {dsStatus.label}
          </span>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  /* 180 宽、高 60/80（后端按 MiMo 卡片有无自适应）；body 已透明（见 app.css），卡片自身做半透明质感。 */
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

  /* GLM / MiniMax 行：两条 4px 进度条占满右侧（上=5小时，下=本周），不显示百分比文字，
     具体数值放在行 tooltip；整体 16px 高，低于每行 20px 的窗口预算，
     因此不增加悬浮条高度。 */
  .dual-row {
    grid-template-columns: 46px 1fr;
  }
  .tracks {
    display: grid;
    grid-template-rows: 1fr 1fr;
    height: 16px;
    align-items: center;
  }
  .track.thin {
    height: 4px;
    border-radius: 2px;
  }

  /* DeepSeek 余额行：无进度条（余额没有分母），brand 列加宽放下 8 字母，
     金额跨满剩余空间右对齐。 */
  .balance-row {
    grid-template-columns: 52px 1fr;
  }
  .balance-right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    min-width: 0;
  }
  .balance-row .amount {
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .status-badge {
    flex: none;
    padding: 1px 4px;
    border: 1px solid currentColor;
    border-radius: 999px;
    font-size: 8px;
    font-weight: 600;
    line-height: 1.1;
    white-space: nowrap;
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
    background-repeat: no-repeat;
    transition: width 0.4s ease;
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
