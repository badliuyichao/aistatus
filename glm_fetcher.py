"""GLM 智谱AI Coding Plan 用量查询模块"""

import json
import os
import sys
import time
import urllib.request
import urllib.error
from typing import Optional

GLM_QUOTA_URL = "https://open.bigmodel.cn/api/monitor/usage/quota/limit"
CONFIG_FILE = "config.json"

# Tag for stderr logs — distinguishes this fetcher's output when all three run together.
_LOG_TAG = "[GlmFetcher]"


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


class GlmFetcher:
    """Fetch GLM Coding Plan quota usage from the official API."""

    def __init__(self, api_key: Optional[str] = None):
        self._api_key = api_key or load_config().get("glm_api_key", "")

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
        config["glm_api_key"] = api_key
        save_config(config)

    def fetch(self) -> Optional[dict]:
        """Fetch GLM Coding Plan quota usage from API.

        Returns a dict suitable for merging into balance.json services,
        or None on failure / missing key.
        """
        if not self._api_key:
            return None

        try:
            req = urllib.request.Request(
                GLM_QUOTA_URL,
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
            print(f"{_LOG_TAG} network error: {e.reason}", file=sys.stderr)
            return None
        except json.JSONDecodeError as e:
            print(f"{_LOG_TAG} invalid JSON response: {e}", file=sys.stderr)
            return None
        except OSError as e:
            print(f"{_LOG_TAG} IO error: {e}", file=sys.stderr)
            return None

        # Check API response code
        if body.get("code") != 200 or "data" not in body:
            print(f"{_LOG_TAG} unexpected response: code={body.get('code')} has_data={'data' in body}",
                  file=sys.stderr)
            return None

        # Account-level plan tier (e.g. "max"). Surfaced in the card title so
        # the user can tell which Coding Plan tier this key belongs to.
        level = body["data"].get("level", "")

        limits = body["data"].get("limits", [])
        if not limits:
            # API succeeded but Coding Plan is not active or no quota attached.
            print(f"{_LOG_TAG} no limits in response (Coding Plan not active?)",
                  file=sys.stderr)
            return None

        now_ms = time.time() * 1000  # for "time until reset" suffix

        # Parse quota items from the limits array
        items = []
        tool_item = None  # TIME_LIMIT: tool-call count quota, appended last
        for limit in limits:
            limit_type = limit.get("type", "")
            total_val = limit.get("usage")
            used_val = limit.get("currentValue")
            pct = limit.get("percentage", 0)

            if limit_type == "TIME_LIMIT":
                # Tool-call count quota (search-prime / web-reader / zread …).
                # This entry carries absolute used/total, unlike the pure-%
                # TOKENS_LIMIT rows — surface it as a concrete "次" bar.
                if total_val and total_val > 0:
                    used = used_val or 0
                    reset = _fmt_duration((limit.get("nextResetTime") or 0) - now_ms)
                    tool_item = {
                        "label": "工具调用",
                        "used": used,
                        "total": total_val,
                        "unit": "次",
                        "detail": f"{reset}后重置" if reset else "",
                    }
                continue

            if limit_type == "TOKENS_LIMIT" and pct is not None:
                # Token quota — API may only return percentage without absolute values
                unit = limit.get("unit", 0)
                num = limit.get("number", 0)
                # Determine label from unit/number
                if unit == 3 and num == 5:
                    label = "5小时限额"
                elif unit == 6 and num == 1:
                    label = "周限额"
                else:
                    label = "Token限额"

                if total_val is not None and total_val > 0:
                    # Has absolute values
                    items.append({
                        "label": label,
                        "used": used_val or 0,
                        "total": total_val,
                        "unit": "tokens",
                        "detail": f"已用 {_fmt_tokens(used_val or 0)} / {_fmt_tokens(total_val)} ({pct}%)",
                    })
                else:
                    # Only percentage available
                    reset = _fmt_duration((limit.get("nextResetTime") or 0) - now_ms)
                    items.append({
                        "label": label,
                        "used": pct,
                        "total": 100,
                        "unit": "%",
                        "detail": f"{reset}后重置" if reset else "",
                    })

        if tool_item:
            items.append(tool_item)

        if not items:
            return None

        return {
            "name": "GLM 智谱AI",
            "type": "quota",
            "icon": "🔷",
            "color": "#2563EB",
            "level": level,
            "items": items,
        }


def _fmt_tokens(val: float) -> str:
    """Format token count for human readability."""
    if val >= 100_000_000:
        return f"{val / 100_000_000:.2f}亿"
    elif val >= 10_000:
        return f"{val / 10_000:.1f}万"
    else:
        return str(int(val))


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
