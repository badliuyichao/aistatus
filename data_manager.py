"""Data model and file watcher for AI API balance monitoring."""

import json
import os
import time
from dataclasses import dataclass, field
from typing import Optional

from PySide6.QtCore import QObject, Signal, QFileSystemWatcher, QTimer


# ── Data Models ───────────────────────────────────────────────────────────────

@dataclass
class QuotaItem:
    """A quota item with used/total tracking (e.g. 5-hour limit)."""
    label: str = ""
    used: float = 0.0
    total: float = 1.0
    unit: str = ""
    detail: str = ""

    @property
    def percentage(self) -> float:
        if self.total <= 0:
            return 0.0
        return min(self.used / self.total * 100, 100.0)

    @property
    def remaining(self) -> float:
        return max(self.total - self.used, 0.0)


@dataclass
class ServiceInfo:
    """A service provider entry (GLM, DeepSeek, etc.)."""
    name: str = ""
    type: str = "balance"         # "quota" | "balance"
    icon: str = ""
    color: str = "#888888"
    items: list = field(default_factory=list)   # list[QuotaItem]
    balance: float = 0.0
    currency: str = "¥"
    unit: str = "元"
    detail: str = ""
    # True when no live API data is available yet (startup, or fetch failed).
    # The card then shows N/A instead of stale file values.
    loading: bool = False


@dataclass
class BalanceData:
    """Complete balance data loaded from JSON."""
    title: str = "AI API 余额监控"
    services: list = field(default_factory=list)  # list[ServiceInfo]
    timestamp: float = 0.0

    @property
    def update_time_str(self) -> str:
        t = time.localtime(self.timestamp)
        return time.strftime("%Y-%m-%d %H:%M:%S", t)


# ── Data Loader / Watcher ────────────────────────────────────────────────────

class DataWatcher(QObject):
    """Loads balance.json and emits signal on changes."""

    data_changed = Signal(object)  # BalanceData

    def __init__(self, file_path: str):
        super().__init__()
        self._file_path = os.path.abspath(file_path)
        self._data: Optional[BalanceData] = None

        # Debounce timer — editors (Notepad) fire fileChanged +
        # directoryChanged in a burst, and some rewrite the file
        # (delete + recreate) emitting several more. Collapsing the burst
        # into a single reload 300ms after the last event avoids a flurry
        # of redundant background API refreshes.
        self._debounce_timer = QTimer(self)
        self._debounce_timer.setSingleShot(True)
        self._debounce_timer.setInterval(300)
        self._debounce_timer.timeout.connect(self._do_reload)

        # File system watcher for auto-reload
        self._watcher = QFileSystemWatcher()
        self._update_watch()

    def _update_watch(self):
        dir_path = os.path.dirname(self._file_path)
        if os.path.isfile(self._file_path):
            files = self._watcher.files()
            if self._file_path not in files:
                self._watcher.addPath(self._file_path)
            if dir_path not in self._watcher.directories():
                self._watcher.addPath(dir_path)

        self._watcher.fileChanged.connect(self._on_changed)
        self._watcher.directoryChanged.connect(self._on_changed)

    def _on_changed(self, path=None):
        """Called when balance.json or its directory changes.

        Just re-arms the debounce timer; the actual reload happens in
        ``_do_reload`` once the burst of change events settles. This is
        intentional — a single Notepad save can fire 2-4 events.
        """
        self._debounce_timer.start()

    def _do_reload(self):
        """Reload balance.json and emit ``data_changed`` (debounced)."""
        # Re-add path in case the file was replaced (save-as / overwrite)
        # and dropped out of the watcher.
        if self._file_path not in self._watcher.files():
            self._watcher.addPath(self._file_path)
        if os.path.exists(self._file_path):
            data = self._load_file()
            self._data = data
            self.data_changed.emit(data)

    def load(self) -> BalanceData:
        """Load data from file (initial load)."""
        if os.path.exists(self._file_path):
            self._data = self._load_file()
        else:
            self._data = BalanceData(title="等待数据... (balance.json 未找到)")
        return self._data

    def load_with_deepseek(self, fetcher=None) -> BalanceData:
        """Load from file, then overlay DeepSeek data from API if available."""
        data = self.load()
        if fetcher is None or not fetcher.has_key:
            return data

        try:
            ds_data = fetcher.fetch()
        except Exception:
            ds_data = None

        if ds_data:
            self._merge_deepseek(data, ds_data)
        return data

    def load_with_glm(self, fetcher=None) -> BalanceData:
        """Load from file, then overlay GLM quota data from API if available."""
        data = self.load()
        if fetcher is None or not fetcher.has_key:
            return data

        try:
            glm_data = fetcher.fetch()
        except Exception:
            glm_data = None

        if glm_data:
            self._merge_glm(data, glm_data)
        return data

    def load_with_all(self, deepseek_fetcher=None, glm_fetcher=None,
                      minimax_fetcher=None) -> BalanceData:
        """Load from file, then overlay data from all API fetchers."""
        data = self.load()

        # DeepSeek
        if deepseek_fetcher and deepseek_fetcher.has_key:
            try:
                ds_data = deepseek_fetcher.fetch()
                if ds_data:
                    self._merge_deepseek(data, ds_data)
            except Exception:
                pass

        # GLM
        if glm_fetcher and glm_fetcher.has_key:
            try:
                glm_data = glm_fetcher.fetch()
                if glm_data:
                    self._merge_glm(data, glm_data)
            except Exception:
                pass

        # MiniMax Coding Plan
        if minimax_fetcher and minimax_fetcher.has_key:
            try:
                mm_data = minimax_fetcher.fetch()
                if mm_data:
                    self._merge_minimax(data, mm_data)
            except Exception:
                pass

        return data

    def merge_api_into(self, data: BalanceData, ds_data=None, glm_data=None,
                       mm_data=None) -> BalanceData:
        """Merge pre-fetched API dicts into ``data`` in place.

        Used by the async fetch path: the worker thread fetches raw dicts,
        hands them back here, and only the (fast) file ``load()`` + merge
        + render happens on the GUI thread.

        Always calls all three mergers — a ``None`` argument means that
        source has no live data, and the merger flags the matching service
        as ``loading`` so its card shows N/A rather than a stale file value.
        """
        self._merge_deepseek(data, ds_data)
        self._merge_glm(data, glm_data)
        self._merge_minimax(data, mm_data)
        return data

    @staticmethod
    def _merge_deepseek(data: BalanceData, ds_data):
        """Replace or insert DeepSeek service entry with API-fetched data.

        ``ds_data`` is None when the fetch failed or there is no key — in
        that case a matching entry is flagged ``loading`` so its card shows
        N/A instead of the stale file value.
        """
        # Find existing DeepSeek entry
        for svc in data.services:
            if svc.name == "DeepSeek":
                if ds_data is None:
                    svc.loading = True
                    return
                svc.type = ds_data.get("type", "balance")
                svc.balance = float(ds_data.get("balance", 0))
                svc.currency = ds_data.get("currency", "¥")
                svc.unit = ds_data.get("unit", "元")
                svc.detail = ds_data.get("detail", "")
                svc.color = ds_data.get("color", "#4D6BFE")
                svc.loading = False
                return

        # No existing DeepSeek entry → append one
        if isinstance(ds_data, dict) and ds_data.get("name") == "DeepSeek":
            info = ServiceInfo(
                name="DeepSeek",
                type=ds_data.get("type", "balance"),
                icon=ds_data.get("icon", "🟢"),
                color=ds_data.get("color", "#4D6BFE"),
                balance=float(ds_data.get("balance", 0)),
                currency=ds_data.get("currency", "¥"),
                unit=ds_data.get("unit", "元"),
                detail=ds_data.get("detail", ""),
            )
            data.services.append(info)

    @staticmethod
    def _merge_glm(data: BalanceData, glm_data):
        """Replace or insert GLM service entry with API-fetched quota data.

        ``glm_data`` is None on fetch failure / no key → flag loading (N/A).
        """
        # Find existing GLM entry first so we can flag it loading even when
        # glm_data is None. Match the canonical name OR a previously suffixed
        # form (e.g. "GLM 智谱AI · MAX") so re-refresh still hits.
        for svc in data.services:
            if svc.name == "GLM 智谱AI" or svc.name.startswith("GLM 智谱AI · "):
                if glm_data is None:
                    svc.loading = True
                    return
                # Parse items from API data
                api_items = []
                for raw in glm_data.get("items", []):
                    api_items.append(QuotaItem(
                        label=raw.get("label", ""),
                        used=float(raw.get("used", 0)),
                        total=float(raw.get("total", 1)),
                        unit=raw.get("unit", ""),
                        detail=raw.get("detail", ""),
                    ))
                if not api_items:
                    svc.loading = True
                    return

                # Plan tier → card title suffix (e.g. "GLM 智谱AI · MAX").
                level = glm_data.get("level")
                display_name = f"GLM 智谱AI · {level.upper()}" if level else "GLM 智谱AI"
                svc.type = "quota"
                svc.items = api_items
                svc.color = glm_data.get("color", "#2563EB")
                svc.icon = glm_data.get("icon", svc.icon)
                svc.name = display_name
                svc.loading = False
                return

        # No existing GLM entry → append one
        if not glm_data:
            return
        api_items = []
        for raw in glm_data.get("items", []):
            api_items.append(QuotaItem(
                label=raw.get("label", ""),
                used=float(raw.get("used", 0)),
                total=float(raw.get("total", 1)),
                unit=raw.get("unit", ""),
                detail=raw.get("detail", ""),
            ))
        if not api_items:
            return
        level = glm_data.get("level")
        display_name = f"GLM 智谱AI · {level.upper()}" if level else "GLM 智谱AI"
        info = ServiceInfo(
            name=display_name,
            type="quota",
            icon=glm_data.get("icon", "🔷"),
            color=glm_data.get("color", "#2563EB"),
            items=api_items,
        )
        data.services.append(info)

    @staticmethod
    def _merge_minimax(data: BalanceData, mm_data):
        """Replace or insert MiniMax service entry; mark loading when no API data."""
        # Find existing MiniMax entry first so we can flag it loading when
        # mm_data is None. Match canonical name OR legacy "MiniMax 稀宇科技".
        target_names = ("MiniMax", "MiniMax 稀宇科技")
        for svc in data.services:
            if svc.name in target_names:
                if mm_data is None:
                    svc.loading = True
                    return
                api_items = []
                for raw in mm_data.get("items", []):
                    api_items.append(QuotaItem(
                        label=raw.get("label", ""),
                        used=float(raw.get("used", 0)),
                        total=float(raw.get("total", 1)),
                        unit=raw.get("unit", ""),
                        detail=raw.get("detail", ""),
                    ))
                if not api_items:
                    svc.loading = True
                    return
                svc.type = "quota"
                svc.items = api_items
                svc.color = mm_data.get("color", "#EA580C")
                svc.icon = mm_data.get("icon", svc.icon)
                svc.loading = False
                return

        # No existing MiniMax entry → append one
        if not mm_data:
            return
        api_items = []
        for raw in mm_data.get("items", []):
            api_items.append(QuotaItem(
                label=raw.get("label", ""),
                used=float(raw.get("used", 0)),
                total=float(raw.get("total", 1)),
                unit=raw.get("unit", ""),
                detail=raw.get("detail", ""),
            ))
        if not api_items:
            return
        info = ServiceInfo(
            name="MiniMax",
            type="quota",
            icon=mm_data.get("icon", "🟠"),
            color=mm_data.get("color", "#EA580C"),
            items=api_items,
        )
        data.services.append(info)

    @property
    def data(self) -> Optional[BalanceData]:
        return self._data

    def _load_file(self) -> BalanceData:
        try:
            with open(self._file_path, "r", encoding="utf-8") as f:
                raw = json.load(f)
            result = self._parse(raw)
            result.timestamp = time.time()
            return result
        except json.JSONDecodeError as e:
            return BalanceData(title=f"JSON 解析错误: {e}", timestamp=time.time())
        except Exception as e:
            return BalanceData(title=f"读取失败: {e}", timestamp=time.time())

    @staticmethod
    def _parse(raw: dict) -> BalanceData:
        result = BalanceData(title=raw.get("title", "AI API 余额监控"))
        for svc in raw.get("services", []):
            info = ServiceInfo(
                name=svc.get("name", ""),
                type=svc.get("type", "balance"),
                icon=svc.get("icon", ""),
                color=svc.get("color", "#888888"),
            )
            if info.type == "quota":
                for item in svc.get("items", []):
                    info.items.append(QuotaItem(
                        label=item.get("label", ""),
                        used=float(item.get("used", 0)),
                        total=float(item.get("total", 1)),
                        unit=item.get("unit", ""),
                        detail=item.get("detail", ""),
                    ))
            else:
                info.balance = float(svc.get("balance", 0))
                info.currency = svc.get("currency", "¥")
                info.unit = svc.get("unit", "元")
                info.detail = svc.get("detail", "")
            result.services.append(info)
        return result
