"""DeepSeek API 余额查询模块"""

import json
import os
import sys
import time
import urllib.request
import urllib.error
from typing import Optional

from paths import data_path, is_frozen

DEEPSEEK_BALANCE_URL = "https://api.deepseek.com/user/balance"
CONFIG_FILE = "config.json"

# Tag for stderr logs — distinguishes this fetcher's output when all three run together.
_LOG_TAG = "[DeepSeekFetcher]"


def _config_path() -> str:
    return data_path(CONFIG_FILE)


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
    # .gitignore hint (dev only — no point beside a packaged exe)
    if not is_frozen():
        gitignore = os.path.join(os.path.dirname(path), ".gitignore")
        if not os.path.isfile(gitignore):
            with open(gitignore, "w", encoding="utf-8") as f:
                f.write("config.json\n")


class DeepSeekFetcher:
    """Fetch DeepSeek account balance from the official API."""

    def __init__(self, api_key: Optional[str] = None):
        self._api_key = api_key or load_config().get("deepseek_api_key", "")

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
        config["deepseek_api_key"] = api_key
        save_config(config)

    def fetch(self) -> Optional[dict]:
        """Fetch DeepSeek balance info from API.

        Returns a dict suitable for merging into balance.json services,
        or None on failure / missing key.
        """
        if not self._api_key:
            return None

        try:
            req = urllib.request.Request(
                DEEPSEEK_BALANCE_URL,
                headers={
                    "Accept": "application/json",
                    "Authorization": f"Bearer {self._api_key}",
                },
            )
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8"))
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

        if not data.get("is_available", False):
            # Surface as a red card in the UI; log so we know why the card is red.
            print(f"{_LOG_TAG} account unavailable or balance exhausted", file=sys.stderr)
            return {
                "name": "DeepSeek",
                "type": "balance",
                "icon": "🟢",
                "balance": 0,
                "currency": "¥",
                "unit": "元",
                "detail": "账户不可用或额度耗尽",
                "color": "#FF5252",
            }

        total = 0.0
        currency = "¥"
        if data.get("balance_infos"):
            info = data["balance_infos"][0]
            total = float(info.get("total_balance", 0))
            currency = info.get("currency", "¥")
        else:
            # API succeeded, account is "available", but no balance info attached.
            # Returning ¥0.00 silently is misleading; log it.
            print(f"{_LOG_TAG} no balance_infos in response (unexpected shape)",
                  file=sys.stderr)

        return {
            "name": "DeepSeek",
            "type": "balance",
            "icon": "🟢",
            "balance": total,
            "currency": "¥" if currency in ("CNY", "¥") else currency,
            "unit": "元",
            "detail": f"剩余 ¥{total:,.2f}",
            "color": "#4D6BFE" if total > 10 else "#FF5252",
        }
