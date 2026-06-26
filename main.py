"""AI API 余额监控悬浮框 — 入口"""

import sys
import os

from PySide6.QtWidgets import QApplication
from PySide6.QtCore import Qt, QTimer

from data_manager import DataWatcher
from deepseek_fetcher import DeepSeekFetcher
from glm_fetcher import GlmFetcher
from minimax_fetcher import MiniMaxFetcher
from widget import MainWidget


def main():
    # High DPI support
    QApplication.setHighDpiScaleFactorRoundingPolicy(
        Qt.HighDpiScaleFactorRoundingPolicy.PassThrough
    )

    app = QApplication(sys.argv)
    app.setApplicationName("AI API Balance Monitor")
    app.setOrganizationName("AIStatus")
    app.setQuitOnLastWindowClosed(False)  # Keep running in system tray

    base_dir = os.path.dirname(os.path.abspath(__file__))
    json_path = os.path.join(base_dir, "balance.json")

    # Data watcher
    watcher = DataWatcher(json_path)

    # API fetchers (reads API keys from config.json)
    deepseek = DeepSeekFetcher()
    glm = GlmFetcher()
    minimax = MiniMaxFetcher()

    # Widget
    widget = MainWidget(watcher, deepseek_fetcher=deepseek, glm_fetcher=glm,
                        minimax_fetcher=minimax)
    widget.show()

    # Periodically refresh all API data (every 1 minute). Runs on the widget's
    # background thread, so this never blocks the GUI.
    refresh_timer = QTimer()
    refresh_timer.timeout.connect(widget.request_refresh)
    refresh_timer.start(60_000)  # 1 min

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
