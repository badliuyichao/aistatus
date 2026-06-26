"""MiniMax 稀宇科技 Coding Plan 用量查询模块"""

import json
import os
import sys
import time
import urllib.request
import urllib.error
from typing import Optional

MINIMAX_QUOTA_URL = "https://www.minimaxi.com/v1/api/openplatform/coding_plan/remains"
CONFIG_FILE = "config.json"

# Tag for stderr logs — distinguishes this fetcher's output when all three run together.
_LOG_TAG = "[MiniMaxFetcher]"


def _config_path() -> str:
    return os.path.join(os.path.dirname(os.path.abspath(__file__)), CONFIG_FILE)


def load_config() -> dict:
    """Load config.json (API keys etc.)."""
    path = _config_path()
    if os.path.isfile(path):
        try:
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)
        except (json.JSONDecodeError, OSError):
            return {}
    return {}


def save_config(config: dict):
    """Save config.json."""
    path = _config_path()
    with open(path, "w", encoding="utf-8") as f:
        json.dump(config, f, indent=2, ensure_ascii=False)
    # .gitignore hint
    gitignore = os.path.join(os.path.dirname(path), ".gitignore")
    if not os.path.isfile(gitignore):
        with open(gitignore, "w", encoding="utf-8") as f:
            f.write("config.json\n")


class MiniMaxFetcher:
    """Fetch MiniMax Coding Plan quota usage from the official API.

    Reference: https://platform.minimaxi.com/user-center/payment/coding-plan
    Endpoint: GET /v1/api/openplatform/coding_plan/remains
    Auth:     Bearer <api_key>
    """

    def __init__(self, api_key: Optional[str] = None):
        self._api_key = api_key or load_config().get("minimax_api_key", "")

    @property
    def has_key(self) -> bool:
        return bool(self._api_key)

    @property
    def api_key(self) -> str:
        return self._api_key

    def set_key(self, api_key: str):
        """Update API key and persist to config.json."""
        self._api_key = api_key
        config = load_config()
        config["minimax_api_key"] = api_key
        save_config(config)

    def fetch(self) -> Optional[dict]:
        """Fetch MiniMax Coding Plan quota usage from API.

        Returns a dict suitable for merging into balance.json services,
        or None on failure / missing key.
        """
        if not self._api_key:
            return None

        try:
            req = urllib.request.Request(
                MINIMAX_QUOTA_URL,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self._api_key}",
                },
            )
            with urllib.request.urlopen(req, timeout=10) as resp:
                body = json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            # 4xx / 5xx — usually 401 (key invalid) or 429 (rate limit)
            print(f"{_LOG_TAG} HTTP {e.code} {e.reason} (check api_key / rate limit)",
                  file=sys.stderr)
            return None
        except urllib.error.URLError as e:
            # DNS failure, connection refused, timeout, etc.
            print(f"{_LOG_TAG} network error: {e.reason}", file=sys.stderr)
            return None
        except json.JSONDecodeError as e:
            print(f"{_LOG_TAG} invalid JSON response: {e}", file=sys.stderr)
            return None
        except OSError as e:
            # Socket / timeout that didn't surface as URLError
            print(f"{_LOG_TAG} IO error: {e}", file=sys.stderr)
            return None

        # Check API response code (MiniMax uses base_resp.status_code == 0)
        base_resp = body.get("base_resp", {})
        if base_resp and base_resp.get("status_code") not in (0, None):
            code = base_resp.get("status_code")
            print(f"{_LOG_TAG} API error in base_resp: {base_resp}", file=sys.stderr)
            # Placeholder instead of None → card explains the outage, not blank.
            return {
                "name": "MiniMax", "type": "quota", "icon": "🟠", "color": "#EA580C",
                "items": _placeholder_items(f"API 错误 {code}"),
            }

        model_remains = body.get("model_remains") or []
        if not model_remains:
            # API call succeeded but returned no model data (plan not active, or
            # transient outage). `or []` guards against an explicit JSON null,
            # which dict.get("k", []) would pass through as None.
            print(f"{_LOG_TAG} no model_remains in response (plan not active?)",
                  file=sys.stderr)
            return {
                "name": "MiniMax", "type": "quota", "icon": "🟠", "color": "#EA580C",
                "items": _placeholder_items("暂无用量数据"),
            }

        # Aggregate each window across all *active* models. Strategy: ignore
        # *_total_count / *_usage_count — the API returns zeros there — and
        # trust *_remaining_percent + *_status instead.
        items = [
            _aggregate_window(
                model_remains,
                status_field="current_interval_status",
                pct_field="current_interval_remaining_percent",
                remains_field="remains_time",
                label="5小时限额",
            ),
            _aggregate_window(
                model_remains,
                status_field="current_weekly_status",
                pct_field="current_weekly_remaining_percent",
                remains_field="weekly_remains_time",
                label="周限额",
                # MiniMax-specific: when no model is tracked in the weekly
                # window, the API treats it as "no quota cap". Render as
                # a fully-filled 无限额 bar instead of the generic 未活跃
                # placeholder — visually conveys "nothing to worry about".
                inactive_used=100,
                inactive_detail="无限额",
            ),
        ]

        # Pick a representative model name (active 5h preferred, else first
        # model overall). Internal hint only — _merge_minimax ignores it.
        active_now = [m for m in model_remains if m.get("current_interval_status") == 1]
        if active_now:
            model_name = active_now[0].get("model_name", "")
        elif model_remains:
            model_name = model_remains[0].get("model_name", "")
        else:
            model_name = ""

        return {
            "name": "MiniMax",
            "type": "quota",
            "icon": "🟠",
            "color": "#EA580C",
            "items": items,
            "_model": model_name,  # internal hint, stripped before merge
        }


def _placeholder_items(msg: str) -> list:
    """Build placeholder quota rows for when the API errors or returns no data.

    Used instead of returning None so the card surfaces a clear status
    ('API 错误 1033' / '暂无用量数据') instead of silently going blank.
    """
    return [
        {"label": "5小时限额", "used": 0, "total": 100, "unit": "%", "detail": msg},
        {"label": "周限额", "used": 0, "total": 100, "unit": "%", "detail": msg},
    ]


def _aggregate_window(model_remains, status_field, pct_field, remains_field, label,
                     inactive_used=0, inactive_detail="本周期未开始或无用量"):
    """Build one quota row by aggregating across all active models in a window.

    Plan A: ignore *_total_count / *_usage_count (API returns zeros there)
    and use *_remaining_percent + *_status instead.

    - status == 1   → model is active in this window, include in aggregate.
    - status != 1   → model is not tracked, skip.
    - No active     → emit an "inactive" row. Default is used=0 with the
                      generic "本周期未开始或无用量" detail; callers can
                      override inactive_used / inactive_detail to render
                      a custom state (e.g. MiniMax weekly → 无限额 full-bar).

    Returns a dict shaped like a QuotaItem row:
        {label, used, total=100, unit="%", detail}
    """
    active = [
        m for m in model_remains
        if m.get(status_field) == 1
        and isinstance(m.get(pct_field), (int, float))
    ]

    if not active:
        return {
            "label": label,
            "used": inactive_used,
            "total": 100,
            "unit": "%",
            "detail": inactive_detail,
        }

    # Average used% across active models. (100 - remaining%) gives used%.
    used_pcts = [100.0 - float(m[pct_field]) for m in active]
    used_pct = sum(used_pcts) / len(used_pcts)

    # Time until reset: shortest remaining across active models = earliest reset.
    reset_dur = ""
    if remains_field:
        rems = [m.get(remains_field) for m in active
                if isinstance(m.get(remains_field), (int, float))]
        if rems:
            reset_dur = _fmt_duration(min(rems))

    model_names = sorted({m.get("model_name", "") for m in active if m.get("model_name")})
    model_hint = f" ({', '.join(model_names)})" if model_names else ""

    if reset_dur:
        detail = f"{reset_dur}后重置{model_hint}"
    else:
        detail = ""

    return {
        "label": label,
        "used": round(used_pct, 1),
        "total": 100,
        "unit": "%",
        "detail": detail,
    }


def _fmt_duration(ms) -> str:
    """Format a millisecond duration as a short 'time until reset' string.

    Returns '' for missing/non-positive values so callers can skip the suffix.
    """
    if not ms or ms <= 0:
        return ""
    s = ms / 1000.0
    d = int(s // 86400)
    h = int((s % 86400) // 3600)
    m = int((s % 3600) // 60)
    if d > 0:
        return f"{d}天{h}小时"
    if h > 0:
        return f"{h}小时{m}分"
    return f"{m}分"