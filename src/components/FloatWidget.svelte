<script lang="ts">
  // 悬浮条：长方形小窗，数据驱动渲染所有可见服务实例（同品牌多实例各占一行）：
  //   - quota 型：items 含「5小时限额」+「周限额」两条 → 同一行上下双 4px 进度条
  //     （GLM / MiniMax）；其余（MiMo 的「Token Plan 套餐」等）→ 单条 + 右侧百分比。
  //   - balance 型（DeepSeek）：不放进度条（余额无分母），右对齐显示金额；
  //     金额旁状态标签（安全 ≥50 绿 / 预警 10–<50 黄 / 危险 <10 红），
  //     错误占位（key 无效 / 服务繁忙）不显示状态；明细进 tooltip。
  //
  // 与主窗口共用同一份 index.html，经 main.ts 按 webview label 分流挂载。
  // 复用 services store（自动收 services-updated 广播）和 theme store（跟随主题）。
  //
  // 行标签 = 实例 label（用户命名）|| 品牌短名；行归属按 services 数组顺序
  // （= 设置里 plan 列表顺序）。被隐藏的实例不会出现在数据里（后端出口过滤）。
  //   - 百分比统一按 used/total 计算：GLM/MiniMax 的百分比条 total=100；
  //     GLM 若返回 token 绝对值条（unit=="tokens"）也能得到正确比例。
  //   - MiMo：item.used / total 是**原始额度**（如 893M / 4.1B，非百分比），
  //     比例需自行计算 used/total；明细（绝对值 + 已用%）放行 tooltip。
  //   - item.displayValue === "∞" 表示该窗口未计费/不限额，进度条满格灰色 + ∞ 标记。
  //   - 进度条填充用同色相「浅→深」渐变（barGradient）：渐变按整条 track 铺开，
  //     宽度越大露出的颜色越深。色相按服务商固定（barColors），同品牌多实例
  //     共用色相、靠行标签区分；只有 ≥80% 接近耗尽时统一转红告警。
  //   - 服务 loading 或无对应 item 时显示 N/A。
  //
  // 窗口高度由后端 fit_float_window 按可见行数自适应（基础 20 + 每行 20 逻辑像素）。

  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { services } from "../stores/services.svelte";
  import { theme } from "../stores/theme.svelte";
  import { saveFloatPosition } from "../api";
  import { balanceStatus } from "../balanceStatus";
  import { barGradient, gradientSize } from "../barColors";
  import type { QuotaItem, ServiceInfo } from "../types";

  const appWindow = getCurrentWindow();
  let moveTimer: number | undefined;
  let unlistenMoved: (() => void) | undefined;

  /** 可见服务按类型分组（顺序 = plan 列表顺序，同品牌多实例各占一行）。 */
  const quotaSvcs = $derived(services.data.services.filter((s) => s.type === "quota"));
  const balanceSvcs = $derived(services.data.services.filter((s) => s.type === "balance"));

  /** 行标签：品牌短名 + 实例备注（如「GLM 主账号」），无备注就是品牌短名——
   *  备注随便起名（主账号 / 部门 / 备用），品牌必须可见才能分清是哪一家的。 */
  function rowLabel(svc: ServiceInfo): string {
    const brand = brandShort(svc.vendor);
    const label = svc.label?.trim();
    return label ? `${brand} ${label}` : brand;
  }

  function brandShort(vendor: string): string {
    switch (vendor) {
      case "glm":
        return "GLM";
      case "minimax":
        return "MiniMax";
      case "mimo":
        return "MiMo";
      case "deepseek":
        return "DeepSeek";
      default:
        return vendor;
    }
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
  {#each quotaSvcs as svc (svc.id)}
    {@const items = svc.loading ? [] : svc.items}
    {@const five = items.find((it) => it.label === "5小时限额")}
    {@const week = items.find((it) => it.label === "周限额")}
    {#if five && week}
      <!-- 双进度条行（GLM / MiniMax）：上=5小时、下=本周，数值放行 tooltip -->
      {@const fivePct = usedPct(five)}
      {@const weekPct = usedPct(week)}
      <div
        class="row dual-row"
        title="{rowLabel(svc)} · 5小时限额 {pctLabel(five, fivePct)} · 本周 {pctLabel(week, weekPct)}"
      >
        <span class="brand" style="color:{svc.color || '#888'}">{rowLabel(svc)}</span>
        <div class="tracks">
          <div class="track thin">
            {#if fivePct === null && five.displayValue === "∞"}
              <div class="fill infinite" style="width:100%"></div>
            {:else if fivePct !== null}
              <div
                class="fill"
                style="width:{fivePct}%; background-image:{barGradient(fivePct, svc)}; background-size:{gradientSize(fivePct)}"
              ></div>
            {/if}
          </div>
          <div class="track thin">
            {#if weekPct === null && week.displayValue === "∞"}
              <div class="fill infinite" style="width:100%"></div>
            {:else if weekPct !== null}
              <div
                class="fill"
                style="width:{weekPct}%; background-image:{barGradient(weekPct, svc, 'week')}; background-size:{gradientSize(weekPct)}"
              ></div>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <!-- 单条行（MiMo Token Plan 等）：进度条 + 右侧百分比 -->
      {@const item = items[0]}
      {@const pct = usedPct(item)}
      <div
        class="row"
        title="{rowLabel(svc)}：{item?.displayValue ?? '—'} · {item?.detail ?? '刷新中'}"
      >
        <span class="brand" style="color:{svc.color || '#888'}">{rowLabel(svc)}</span>
        <div class="track">
          {#if pct !== null}
            <div
              class="fill"
              style="width:{pct}%; background-image:{barGradient(pct, svc)}; background-size:{gradientSize(pct)}"
            ></div>
          {/if}
        </div>
        <span class="pct">{pct === null ? "—" : `${Math.round(pct)}%`}</span>
      </div>
    {/if}
  {/each}

  {#each balanceSvcs as svc (svc.id)}
    <!-- 余额行（DeepSeek）：无进度条，右对齐金额 + 状态标签 -->
    {@const status = svc.loading || svc.placeholder ? null : balanceStatus(svc.balance)}
    <div
      class="row balance-row"
      title="{rowLabel(svc)} 余额：{svc.loading ? '刷新中' : `${svc.currency}${fmtAmount(svc.balance)}`}{status ? ` · ${status.label}` : ''}{svc.detail ? ` · ${svc.detail}` : ''}"
    >
      <span class="brand" style="color:{svc.color || '#4D6BFE'}">{rowLabel(svc)}</span>
      <div class="balance-right">
        <span
          class="amount"
          style="color:{svc.loading ? 'var(--text-muted)' : status?.color ?? 'var(--text-muted)'}"
        >
          {svc.loading ? "—" : `${svc.currency}${fmtAmount(svc.balance)}`}
        </span>
        {#if status}
          <span class="status-badge" style="color:{status.color}; border-color:{status.color}">
            {status.label}
          </span>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  /* 220 宽、高按可见行数自适应（后端 fit_float_window）；body 已透明（见 app.css），
     卡片自身做半透明质感。 */
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
    /* 品牌列统一固定宽度：各行进度条起点对齐；超长备注截断省略 */
    grid-template-columns: 76px 1fr 30px;
    align-items: center;
    gap: 6px;
  }

  /* 双进度条行：两条 4px 进度条占满右侧（上=5小时，下=本周），不显示百分比文字，
     具体数值放在行 tooltip；整体 16px 高，低于每行 20px 的窗口预算，
     因此不增加悬浮条高度。 */
  .dual-row {
    grid-template-columns: 76px 1fr;
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

  /* 余额行：无进度条（余额没有分母），金额跨满剩余空间右对齐。 */
  .balance-row {
    grid-template-columns: 76px 1fr;
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
    /* 列宽固定 76px。min-width:0 必须显式声明：grid 项默认 min-width:auto，
       nowrap 长文字会把项撑出列外压到进度条，截断也随之失效 */
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
