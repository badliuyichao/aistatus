"""Filesystem path resolution for dev and PyInstaller-frozen builds.

Two kinds of paths matter once the app is packaged:

* **Read-only resources** — `icons/`, `aistatus.ico`. These are bundled into
  the PyInstaller archive; at runtime they live in ``sys._MEIPASS`` (the temp
  extraction dir for ``--onefile``, or the bundle dir for ``--onedir``).
* **Writable user data** — `config.json` (API keys, written by the settings
  dialog) and `balance.json` (user-edited display data). These must persist
  across runs and reinstalls, so when frozen they live in the per-user
  app-data dir (Windows: ``%APPDATA%\\aistatus``); next to the script in dev.

Mixing the two (e.g. writing config.json under ``_MEIPASS`` in a ``--onefile``
build) silently loses data, because ``_MEIPASS`` is wiped on exit.
"""

import os
import shutil
import sys


def is_frozen() -> bool:
    """True when running inside a PyInstaller bundle."""
    return getattr(sys, "frozen", False) and hasattr(sys, "_MEIPASS")


def resource_dir() -> str:
    """Directory of read-only bundled resources (icons, app icon, templates)."""
    if is_frozen():
        return sys._MEIPASS  # type: ignore[attr-defined]
    return os.path.dirname(os.path.abspath(__file__))


def data_dir() -> str:
    """Directory of writable user data (config.json keys, balance.json data).

    Frozen (packaged exe): the per-user app-data dir (Windows:
    ``%APPDATA%\\aistatus``) so API keys persist across reinstalls and the app
    still works installed to a read-only location like Program Files. Created
    on first use. Dev: next to the script, where the project's sample files
    live.
    """
    if is_frozen():
        base = os.environ.get("APPDATA") or os.path.expanduser("~")
        d = os.path.join(base, "aistatus")
        os.makedirs(d, exist_ok=True)
        return d
    return os.path.dirname(os.path.abspath(__file__))


def resource_path(*parts: str) -> str:
    """Absolute path to a read-only bundled resource."""
    return os.path.join(resource_dir(), *parts)


def data_path(*parts: str) -> str:
    """Absolute path to a writable user-data file."""
    return os.path.join(data_dir(), *parts)


def ensure_user_data(filename: str) -> str:
    """Return the writable path for ``filename``, seeding it on first run.

    In a ``--onefile`` build the bundled template lives in the read-only
    ``_MEIPASS``; the writable copy the app (and user) actually uses lives next
    to the exe. On first run that copy doesn't exist yet, so copy the template
    out of the bundle. No-op when the writable file already exists, and in dev
    / ``--onedir`` where the two locations coincide.
    """
    target = data_path(filename)
    if not os.path.exists(target):
        template = resource_path(filename)
        if os.path.exists(template):
            shutil.copy2(template, target)
    return target
