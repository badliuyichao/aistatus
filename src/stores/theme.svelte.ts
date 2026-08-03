// 主题状态：偏好（system/dark/light）+ 已解析（dark/light）。
//
// Svelte 5 rune：pref/resolved 用 $state 暴露；配色实际由 <html data-theme>
// 驱动的 CSS 变量负责，组件不用读 resolved。
//
// system 模式监听 matchMedia('prefers-color-scheme')，系统主题变化时自动重解析。
// 窗口默认隐藏 → init 在用户唤起前完成，无主题闪烁。

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getTheme, setTheme } from "../api";
import type { Theme } from "../types";

type Resolved = "dark" | "light";

class ThemeStore {
  pref = $state<Theme>("system");
  resolved = $state<Resolved>("light");

  private mql: MediaQueryList | undefined;
  private unlistenTheme?: UnlistenFn;
  private inited = false;

  /** 启动时调一次：读偏好 + 应用 + 监听系统主题 + 监听跨窗口同步。重复调用忽略。 */
  async init() {
    if (this.inited) return;
    this.inited = true;
    try {
      this.pref = await getTheme();
    } catch (e) {
      console.error("get_theme failed", e);
    }
    this.mql = window.matchMedia("(prefers-color-scheme: dark)");
    this.mql.addEventListener("change", this.onSystemChange);
    // 跨窗口主题同步：主窗口改主题 → 后端广播 theme-changed → 所有窗口重应用。
    // 主窗口和悬浮条是独立 webview，各自 document 互不可见，必须靠事件同步。
    // 发起切换的窗口自身已 apply，收到事件再 apply 一次幂等无副作用。
    this.unlistenTheme = await listen<Theme>("theme-changed", (e) => {
      this.pref = e.payload;
      this.apply();
    });
    this.apply();
  }

  destroy() {
    this.mql?.removeEventListener("change", this.onSystemChange);
    this.unlistenTheme?.();
  }

  private onSystemChange = () => {
    // 仅 system 模式响应系统变化；强制 dark/light 时忽略。
    if (this.pref === "system") this.apply();
  };

  /** 把 pref 解析为 resolved 并写到 <html data-theme>。 */
  private apply() {
    const resolved: Resolved =
      this.pref === "system" ? (this.mql?.matches ? "dark" : "light") : this.pref;
    this.resolved = resolved;
    document.documentElement.dataset.theme = resolved;
  }

  /** 切换偏好：即时应用（用户立刻看到效果）+ 持久化 + 广播给其他窗口。 */
  async set(pref: Theme) {
    this.pref = pref;
    this.apply();
    try {
      await setTheme(pref);
    } catch (e) {
      console.error("set_theme failed", e);
    }
  }
}

export const theme = new ThemeStore();
