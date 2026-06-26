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

  /** 启动时调一次：拉初始数据 + 订阅后端事件。 */
  async init() {
    try {
      this.data = await getServices();
    } catch (e) {
      console.error("get_services failed", e);
    }
    this.unlisten = await onServicesUpdated((d) => {
      this.data = d;
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
    }
  }

  destroy() {
    this.unlisten?.();
  }
}

export const services = new ServicesStore();
