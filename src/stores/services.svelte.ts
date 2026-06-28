// 服务数据全局状态。
// 对应 legacy widget.py 里 MainWidget 维护的当前 BalanceData + 刷新触发。
//
// Svelte 5 rune：用 $state 暴露响应式数据，组件 import 后直接用。

import {
  getServices,
  onServicesUpdated,
  refreshNow,
} from "../api";
import type { BalanceData } from "../types";

class ServicesStore {
  data = $state<BalanceData>({ title: "AI API 余额监控", services: [] });
  refreshing = $state(false);

  private unlisten?: () => void;
  private inited = false;
  // 用于区分本次 refresh 的兜底定时器，避免旧定时器误清新刷新状态
  private refreshToken = 0;
  private refreshTimer: number | undefined;

  /** 启动时调一次：拉初始数据 + 订阅后端事件。重复调用会被忽略。 */
  async init() {
    if (this.inited) return;
    this.inited = true;
    try {
      this.data = await getServices();
    } catch (e) {
      console.error("get_services failed", e);
    }
    this.unlisten = await onServicesUpdated((d) => {
      this.data = d;
      this.clearRefreshTimer();
      this.refreshing = false;
    });
  }

  /** 手动刷新（F5 / 按钮）。 */
  async refresh() {
    this.refreshing = true;
    try {
      await refreshNow();
    } catch (e) {
      console.error("refresh_now failed", e);
      this.refreshing = false;
      return;
    }
    // 兜底：后端防重入丢弃刷新时不 emit services-updated，refreshing 会卡 true。
    // 这里加超时复位，避免 footer 永久显示「刷新中…」。
    // 正常情况 onServicesUpdated 回调会先把它置 false；超时仅作保底。
    const token = ++this.refreshToken;
    this.refreshTimer = window.setTimeout(() => {
      if (this.refreshToken === token) {
        this.refreshing = false;
      }
    }, 15000);
  }

  private clearRefreshTimer() {
    if (this.refreshTimer !== undefined) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = undefined;
    }
  }

  destroy() {
    this.clearRefreshTimer();
    this.unlisten?.();
  }
}

export const services = new ServicesStore();
