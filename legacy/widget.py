"""Floating desktop widget for AI API balance monitoring."""

import sys
import math
import os
import ctypes
from typing import Optional

from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel, QScrollArea,
    QFrame, QApplication, QMenu, QSystemTrayIcon, QSizePolicy,
    QDialog, QLineEdit, QPushButton, QCheckBox,
)
from PySide6.QtCore import Qt, QTimer, QRectF, QPoint, Signal, QSize, QThread
from PySide6.QtGui import (
    QPainter, QPainterPath, QColor, QBrush, QFont,
    QAction, QIcon, QPixmap, QCursor, QFontMetrics,
    QMouseEvent, QResizeEvent, QLinearGradient, QPalette,
    QShortcut, QKeySequence,
)

from data_manager import BalanceData, ServiceInfo, QuotaItem, DataWatcher
from fetch_worker import FetchWorker
from paths import resource_path


# ── Constants ─────────────────────────────────────────────────────────────────

WINDOW_MIN_WIDTH = 300
WINDOW_MIN_HEIGHT = 200
WINDOW_DEFAULT_WIDTH = 340
# Fixed height: sized to fit all three cards with their reset-time detail
# lines snugly (content ~595 + ~79 chrome = ~674, rounded up).
WINDOW_DEFAULT_HEIGHT = 680
# Gap from the screen / taskbar edges when anchored to the bottom-right.
WINDOW_SCREEN_MARGIN = 20
CORNER_RADIUS = 12
RESIZE_HANDLE_SIZE = 16
TITLEBAR_HEIGHT = 36


# ── Helpers ───────────────────────────────────────────────────────────────────

def _try_enable_acrylic(hwnd: int) -> bool:
    """Enable Windows 10/11 acrylic / mica backdrop effect via DWM API.

    Tries multiple strategies in order:
      1. DWMWA_SYSTEMBACKDROP_TYPE (Win 11 22H2+, build 22621+) – modern acrylic
      2. DWMWA_MICA (Win 11 21H2+, build 22000+) – mica
      3. SetWindowCompositionAttribute (Win 10/11) – classic acrylic
    """
    import ctypes
    from ctypes import wintypes, c_int, c_bool

    # ── Helper: try DwmSetWindowAttribute ──────────────────────────────
    def _try_dwm_attr(attr: int, value: int) -> bool:
        try:
            dwmapi = ctypes.windll.dwmapi
            dwmapi.DwmSetWindowAttribute.argtypes = (
                wintypes.HANDLE, c_int, ctypes.POINTER(c_int), c_int,
            )
            v = c_int(value)
            ok = dwmapi.DwmSetWindowAttribute(hwnd, attr, ctypes.byref(v), ctypes.sizeof(v))
            return ok == 0  # S_OK
        except Exception:
            return False

    # Strategy 1: DWMWA_SYSTEMBACKDROP_TYPE → DWMSBT_TABBEDWINDOW (acrylic, Win 11 22H2+)
    DWMWA_SYSTEMBACKDROP_TYPE = 38
    DWMSBT_TABBEDWINDOW = 2   # acrylic backdrop
    if _try_dwm_attr(DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_TABBEDWINDOW):
        return True

    # Strategy 2: DWMWA_MICA (Win 11 21H2+, lighter than acrylic)
    DWMWA_MICA = 1029
    DWMWA_MICA_ENABLED = 1
    if _try_dwm_attr(DWMWA_MICA, DWMWA_MICA_ENABLED):
        return True

    # Strategy 3: classic SetWindowCompositionAttribute with acrylic
    try:
        class ACCENTPOLICY(ctypes.Structure):
            _fields_ = [
                ("AccentState", ctypes.c_uint),
                ("AccentFlags", ctypes.c_uint),
                ("GradientColor", ctypes.c_uint),
                ("AnimationId", ctypes.c_uint),
            ]

        class WINCOMPATTRDATA(ctypes.Structure):
            _fields_ = [
                ("Attribute", ctypes.c_int),
                ("Data", ctypes.POINTER(ACCENTPOLICY)),
                ("SizeOfData", ctypes.c_size_t),
            ]

        accent = ACCENTPOLICY()
        accent.AccentState = 3          # ACCENT_ENABLE_ACRYLIC_BLUR
        accent.AccentFlags = 0x20       # draw with gradient color
        accent.GradientColor = 0xCC191923  # ABGR tint: dark blue-black

        data = WINCOMPATTRDATA()
        data.Attribute = 19
        data.SizeOfData = ctypes.sizeof(accent)
        data.Data = ctypes.pointer(accent)

        user32 = ctypes.windll.user32
        SetWindowCompositionAttribute = user32.SetWindowCompositionAttribute
        SetWindowCompositionAttribute.argtypes = (
            wintypes.HANDLE,
            ctypes.POINTER(WINCOMPATTRDATA),
        )
        SetWindowCompositionAttribute.restype = ctypes.c_bool
        return bool(SetWindowCompositionAttribute(hwnd, ctypes.byref(data)))
    except Exception:
        return False


# ── Background Container ─────────────────────────────────────────────────────

class BackgroundWidget(QWidget):
    """Draws the rounded-rect window body (the visible card).

    Strategy (two-layer painting to defeat the "white corners" DWM bug):

    1. **Matte layer** — fill the entire widget rectangle with an opaque
       near-black colour (#12121C).  This guarantees every pixel is painted,
       so DWM never sees a fully-transparent region to fill with the system
       colour.

    2. **Card layer** — draw a rounded-rect on top with the translucent
       acrylic-friendly colour.  The anti-aliased edge now blends smoothly
       into the dark matte instead of into transparent → system-colour.
    """

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAttribute(Qt.WA_StyledBackground, False)
        self._bg_color = QColor(221, 228, 239, 253)

    def set_bg_color(self, color: QColor):
        self._bg_color = color
        self.update()

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)

        # ── Layer 1: fill the *entire* widget rect with an opaque dark matte ──
        # This seals every pixel — including the four corners that lie outside
        # the rounded path — so DWM never sees a fully-transparent region to
        # fill with the system colour (white).  Without this matte the corners
        # and the anti-aliased 1px edge of the rounded path would both render
        # white on Windows.
        painter.setPen(Qt.NoPen)
        painter.setBrush(QColor(205, 214, 228))      # light matte (seals corners)
        painter.drawRect(self.rect())

        # ── Layer 2: rounded card body with the translucent acrylic-friendly
        # colour on top.  Its anti-aliased edges now blend smoothly into the
        # dark matte instead of into transparent pixels → DWM white backdrop.
        path = QPainterPath()
        path.addRoundedRect(QRectF(self.rect()), CORNER_RADIUS, CORNER_RADIUS)
        painter.setBrush(self._bg_color)
        painter.drawPath(path)


# ── TitleBar ──────────────────────────────────────────────────────────────────

class TitleBar(QWidget):
    """Custom draggable title bar with close/minimize/pin buttons."""

    close_clicked = Signal()
    minimize_clicked = Signal()
    pin_clicked = Signal()
    config_clicked = Signal()

    def __init__(self, title: str = "📊 AI API 余额监控", parent=None):
        super().__init__(parent)
        self.setFixedHeight(TITLEBAR_HEIGHT)
        self._drag_pos = QPoint()
        self._dragging = False

        layout = QHBoxLayout(self)
        layout.setContentsMargins(12, 0, 4, 0)
        layout.setSpacing(4)

        # Title label
        self._title_label = QLabel(title)
        self._title_label.setStyleSheet("""
            QLabel {
                color: #1E2433;
                font-size: 13px;
                font-weight: 600;
                padding: 0;
            }
        """)
        layout.addWidget(self._title_label, 1)

        # Spacer to push buttons right
        layout.addStretch()

        # Config button (API key settings)
        self._config_btn = QLabel("⚙")
        self._config_btn.setFixedSize(28, 22)
        self._config_btn.setAlignment(Qt.AlignCenter)
        self._config_btn.setAttribute(Qt.WA_StyledBackground, True)
        self._config_btn.setToolTip("配置 API Key")
        self._config_btn.setStyleSheet("""
            QLabel {
                color: #8A92A0;
                font-size: 14px;
                border-radius: 4px;
            }
            QLabel:hover { background: rgba(0,0,0,0.06); color: #1E2433; }
        """)
        self._config_btn.mousePressEvent = lambda e: self.config_clicked.emit() if e.button() == Qt.LeftButton else None
        layout.addWidget(self._config_btn)

        # Pin button
        self._pin_btn = QLabel("📌")
        self._pin_btn.setFixedSize(28, 22)
        self._pin_btn.setAlignment(Qt.AlignCenter)
        self._pin_btn.setAttribute(Qt.WA_StyledBackground, True)
        self._pin_btn.setStyleSheet("""
            QLabel {
                color: #2563EB;
                font-size: 12px;
                border-radius: 4px;
            }
            QLabel:hover { background: rgba(79,135,255,0.2); }
        """)
        self._pin_btn.mousePressEvent = lambda e: self.pin_clicked.emit() if e.button() == Qt.LeftButton else None
        layout.addWidget(self._pin_btn)

        # Minimize button
        self._min_btn = QLabel("─")
        self._min_btn.setFixedSize(28, 22)
        self._min_btn.setAlignment(Qt.AlignCenter)
        self._min_btn.setAttribute(Qt.WA_StyledBackground, True)
        self._min_btn.setStyleSheet("""
            QLabel {
                color: #8A92A0;
                font-size: 14px;
                font-weight: bold;
                border-radius: 4px;
            }
            QLabel:hover { background: rgba(0,0,0,0.06); color: #1E2433; }
        """)
        self._min_btn.mousePressEvent = lambda e: self.minimize_clicked.emit() if e.button() == Qt.LeftButton else None
        layout.addWidget(self._min_btn)

        # Close button
        self._close_btn = QLabel("✕")
        self._close_btn.setFixedSize(28, 22)
        self._close_btn.setAlignment(Qt.AlignCenter)
        self._close_btn.setAttribute(Qt.WA_StyledBackground, True)
        self._close_btn.setStyleSheet("""
            QLabel {
                color: #8A92A0;
                font-size: 13px;
                font-weight: bold;
                border-radius: 4px;
            }
            QLabel:hover { background: rgba(255,60,60,0.3); color: #FF4444; }
        """)
        self._close_btn.mousePressEvent = lambda e: self.close_clicked.emit() if e.button() == Qt.LeftButton else None
        layout.addWidget(self._close_btn)

    def set_title(self, text: str):
        self._title_label.setText(text)

    def set_pinned(self, pinned: bool):
        """Reflect pin state at a glance.

        pinned   → solid blue toggle (active, always-on-top ON)
        unpinned → dim, transparent (inactive)
        Same icon, but the filled-vs-outline background makes the state
        obvious; the old blue-text-vs-grey-text difference was invisible
        on the light card.
        """
        if pinned:
            self._pin_btn.setText("📌")
            self._pin_btn.setStyleSheet("""
                QLabel {
                    color: #FFFFFF;
                    font-size: 12px;
                    background: #2563EB;
                    border-radius: 4px;
                }
                QLabel:hover { background: #1D4ED8; }
            """)
        else:
            self._pin_btn.setText("📌")
            self._pin_btn.setStyleSheet("""
                QLabel {
                    color: #9AA1AD;
                    font-size: 12px;
                    background: transparent;
                    border-radius: 4px;
                }
                QLabel:hover { background: rgba(0,0,0,0.06); color: #586070; }
            """)

    def mousePressEvent(self, event: QMouseEvent):
        if event.button() == Qt.LeftButton:
            self._dragging = True
            self._drag_pos = event.globalPosition().toPoint()
            parent = self.parentWidget()
            while parent and not isinstance(parent, MainWidget):
                parent = parent.parentWidget() if parent else None
            if parent:
                self._parent_pos = parent.pos()
            event.accept()

    def mouseMoveEvent(self, event: QMouseEvent):
        if self._dragging:
            delta = event.globalPosition().toPoint() - self._drag_pos
            parent = self.parentWidget()
            while parent and not isinstance(parent, MainWidget):
                parent = parent.parentWidget() if parent else None
            if parent:
                parent.move(self._parent_pos + delta)

    def mouseReleaseEvent(self, event: QMouseEvent):
        if event.button() == Qt.LeftButton:
            self._dragging = False


# ── Progress Bar Widget ──────────────────────────────────────────────────────

class QuotaProgressBar(QWidget):
    """Custom progress bar with gradient fill and rounded corners."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedHeight(8)
        self._value = 0.0  # 0-100
        self._color = QColor("#4F87FF")

    def set_value(self, pct: float, color: QColor = None):
        self._value = max(0.0, min(pct, 100.0))
        if color:
            self._color = color
        self.update()

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing)

        w = self.width()
        h = self.height()
        r = h / 2

        # Track
        track_path = QPainterPath()
        track_path.addRoundedRect(QRectF(0, 0, w, h), r, r)
        painter.setPen(Qt.NoPen)
        painter.setBrush(QColor(0, 0, 0, 16))
        painter.drawPath(track_path)

        # Fill
        fill_w = max(r * 2, w * self._value / 100.0)
        fill_rect = QRectF(0, 0, fill_w, h)
        fill_path = QPainterPath()
        fill_path.addRoundedRect(fill_rect, r, r)

        # Gradient
        grad = QLinearGradient(0, 0, fill_w, 0)
        c = self._color
        grad.setColorAt(0.0, QColor(c.red(), c.green(), c.blue(), 220))
        grad.setColorAt(1.0, QColor(c.red(), c.green(), c.blue(), 180))

        painter.setBrush(grad)
        painter.drawPath(fill_path)

        # Glow line at top
        if self._value > 3:
            glow_rect = QRectF(2, 1, max(r * 2, fill_w - 4), h / 3)
            glow_path = QPainterPath()
            glow_path.addRoundedRect(glow_rect, 2, 2)
            painter.setBrush(QColor(255, 255, 255, 30))
            painter.drawPath(glow_path)


# ── Service logo loader ─────────────────────────────────────────────────────

_LOGO_PIXMAPS = {}  # path → QPixmap cache

def _service_logo(name: str):
    """Return the brand-logo QPixmap for a service, or None (fall back to emoji).

    Matched by name prefix so the appended tier suffix ('GLM 智谱AI · MAX')
    still resolves to the GLM logo.
    """
    key = (name or "").strip()
    if key.startswith("GLM"):
        fname = "glm.png"
    elif key.startswith("DeepSeek"):
        fname = "deepseek.png"
    elif key.startswith("MiniMax"):
        fname = "minimax.png"
    else:
        return None
    path = resource_path("icons", fname)
    if not os.path.isfile(path):
        return None
    if path not in _LOGO_PIXMAPS:
        pix = QPixmap(path)
        if pix.isNull():
            return None
        _LOGO_PIXMAPS[path] = pix
    return _LOGO_PIXMAPS[path]


# ── Service Card ─────────────────────────────────────────────────────────────

class ServiceCard(QFrame):
    """Card displaying a service provider's quota or balance info.

    The card shell (frame style, header row, content container) is built
    once and reused across updates. Refreshes update labels and progress
    values in place; rows are rebuilt only when the structure changes
    (e.g. a quota item count change), so periodic refreshes don't flicker.
    """

    def __init__(self, service: ServiceInfo, parent=None):
        super().__init__(parent)
        self._service = service
        self._quota_rows = []        # one dict per quota row, current render
        self._balance_widgets = None  # dict of balance widgets, lazily built
        # Signature of the currently-rendered content — ("quota", n) or
        # ("balance",). A change forces a rebuild; equality means we can
        # just refill values in place.
        self._content_sig = None
        self._build_shell()
        self._apply(service)

    # ── Shell (built once) ──────────────────────────────────────────────

    def _build_shell(self):
        self.setObjectName("ServiceCard")
        self.setStyleSheet("""
            #ServiceCard {
                background: rgba(255, 255, 255, 0.96);
                border: 1px solid rgba(0, 0, 0, 0.08);
                border-radius: 10px;
            }
        """)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(14, 12, 14, 12)
        outer.setSpacing(10)

        # Header row: icon + name ... status dot
        self._icon_label = QLabel()
        self._icon_label.setFixedSize(24, 24)
        self._icon_label.setAlignment(Qt.AlignCenter)

        self._name_label = QLabel()

        self._status_label = QLabel()
        self._status_label.setFixedSize(8, 8)

        header = QHBoxLayout()
        header.setSpacing(8)
        header.addWidget(self._icon_label)
        header.addWidget(self._name_label, 1)
        header.addStretch()
        header.addWidget(self._status_label)
        outer.addLayout(header)

        # Content container — its children are managed by _apply_*.
        self._content = QWidget()
        self._content.setAttribute(Qt.WA_StyledBackground, False)
        self._content_vbox = QVBoxLayout(self._content)
        self._content_vbox.setContentsMargins(0, 0, 0, 0)
        self._content_vbox.setSpacing(8)
        outer.addWidget(self._content)

    # ── Update ──────────────────────────────────────────────────────────

    def update_service(self, service: ServiceInfo):
        """Refresh the card in place; rebuilds rows only on structural change."""
        self._apply(service)

    def _apply(self, service: ServiceInfo):
        self._service = service
        self._update_header(service)
        if service.type == "quota":
            self._apply_quota(service)
        else:
            self._apply_balance(service)

    def _update_header(self, s: ServiceInfo):
        color = s.color or "#888"
        # Icon: brand logo if available, else the emoji fallback.
        logo = _service_logo(s.name)
        if logo is not None:
            self._icon_label.setPixmap(
                logo.scaled(22, 22, Qt.KeepAspectRatio, Qt.SmoothTransformation))
            self._icon_label.setText("")
            self._icon_label.setStyleSheet("")  # clear emoji font styling
        else:
            self._icon_label.setPixmap(QPixmap())  # clear any old logo
            self._icon_label.setText(s.icon or "●")
            self._icon_label.setStyleSheet("font-size: 18px;")

        self._name_label.setText(s.name)
        self._name_label.setStyleSheet(
            f"QLabel {{ color: {color}; font-size: 14px; font-weight: 700; }}")

        # Status dot: dimmed while loading (no live data yet).
        status_color = "#C8CDD6" if s.loading else color
        self._status_label.setStyleSheet(
            f"background: {status_color}; border-radius: 4px;")

    def _clear_content(self):
        """Remove and delete every widget currently in the content container."""
        while self._content_vbox.count():
            item = self._content_vbox.takeAt(0)
            w = item.widget()
            if w is not None:
                w.deleteLater()
        self._quota_rows = []
        self._balance_widgets = None

    # ── Quota content ───────────────────────────────────────────────────

    def _apply_quota(self, s: ServiceInfo):
        items = s.items or []
        # Loading state with no item skeleton → one placeholder row.
        if s.loading and not items:
            items = [QuotaItem(label="—")]
        # Same item count → refill in place (the common, zero-flicker path).
        sig = ("quota", len(items))
        if self._content_sig != sig:
            self._clear_content()
            self._quota_rows = [self._make_quota_row() for _ in range(len(items))]
            self._content_sig = sig

        if s.loading:
            # No live data yet — show N/A values against a dimmed bar.
            bar_color = QColor("#C8CDD6")
            for row, item in zip(self._quota_rows, items):
                row["name_label"].setText(item.label)
                row["value_label"].setText("N/A")
                row["bar"].set_value(0, bar_color)
                row["detail_label"].hide()
            return

        bar_color = QColor(s.color if s.color else "#4F87FF")
        for row, item in zip(self._quota_rows, items):
            row["name_label"].setText(item.label)

            used_str = (f"{item.used:.1f}" if isinstance(item.used, float)
                        and item.used != int(item.used) else str(int(item.used)))
            total_str = (f"{item.total:.1f}" if isinstance(item.total, float)
                         and item.total != int(item.total) else str(int(item.total)))
            row["value_label"].setText(f"{used_str}/{total_str} {item.unit}")

            row["bar"].set_value(item.percentage, bar_color)

            if item.detail:
                row["detail_label"].show()
                row["detail_label"].setText(item.detail)
            else:
                row["detail_label"].hide()

    def _make_quota_row(self):
        row = QWidget()
        row.setAttribute(Qt.WA_StyledBackground, False)
        v = QVBoxLayout(row)
        v.setContentsMargins(0, 4, 0, 4)
        v.setSpacing(4)

        label_row = QHBoxLayout()
        label_row.setSpacing(6)
        name_label = QLabel()
        name_label.setStyleSheet("color: #2D3340; font-size: 12px;")
        label_row.addWidget(name_label)
        label_row.addStretch()
        value_label = QLabel()
        value_label.setStyleSheet("color: #586070; font-size: 11px;")
        label_row.addWidget(value_label)
        v.addLayout(label_row)

        bar = QuotaProgressBar()
        v.addWidget(bar)

        detail_label = QLabel()
        detail_label.setStyleSheet("color: #8A92A0; font-size: 10px;")
        v.addWidget(detail_label)
        detail_label.hide()  # shown only when the item has detail text

        self._content_vbox.addWidget(row)
        return {
            "widget": row,
            "name_label": name_label,
            "value_label": value_label,
            "bar": bar,
            "detail_label": detail_label,
        }

    # ── Balance content ─────────────────────────────────────────────────

    def _apply_balance(self, s: ServiceInfo):
        sig = ("balance",)
        if self._content_sig != sig:
            self._clear_content()
            self._build_balance_shell()
            self._content_sig = sig
        self._refill_balance(s)

    def _build_balance_shell(self):
        amount = QWidget()
        amount.setAttribute(Qt.WA_StyledBackground, False)
        h = QHBoxLayout(amount)
        h.setContentsMargins(0, 8, 0, 8)
        h.setSpacing(4)

        cur_label = QLabel()
        cur_label.setAlignment(Qt.AlignBottom)
        h.addWidget(cur_label)

        bal_label = QLabel()
        h.addWidget(bal_label)

        unit_label = QLabel()
        unit_label.setAlignment(Qt.AlignBottom)
        unit_label.setContentsMargins(0, 0, 0, 4)
        h.addWidget(unit_label)

        h.addStretch()
        self._content_vbox.addWidget(amount)

        detail_label = QLabel()
        detail_label.setStyleSheet("color: #8A92A0; font-size: 11px; padding-top: 2px;")
        self._content_vbox.addWidget(detail_label)

        self._balance_widgets = {
            "amount": amount,
            "cur_label": cur_label,
            "bal_label": bal_label,
            "unit_label": unit_label,
            "detail_label": detail_label,
        }

    def _refill_balance(self, s: ServiceInfo):
        w = self._balance_widgets
        if s.loading:
            # No live data yet — show N/A, hide currency/unit/detail.
            w["cur_label"].setVisible(False)
            w["bal_label"].setText("N/A")
            w["bal_label"].setStyleSheet(
                "color: #9AA1AD; font-size: 36px; font-weight: 800;")
            w["unit_label"].setVisible(False)
            w["detail_label"].hide()
            return

        color = s.color or "#888"

        w["cur_label"].setText(s.currency or "")
        w["cur_label"].setVisible(bool(s.currency))
        w["cur_label"].setStyleSheet(
            f"color: {color}; font-size: 18px; font-weight: 600;")

        w["bal_label"].setText(f"{s.balance:,.2f}")
        w["bal_label"].setStyleSheet(
            f"color: {color}; font-size: 36px; font-weight: 800;")

        w["unit_label"].setText(s.unit or "")
        w["unit_label"].setVisible(bool(s.unit))

        if s.detail:
            w["detail_label"].show()
            w["detail_label"].setText(s.detail)
        else:
            w["detail_label"].hide()


# ── Config Dialog ────────────────────────────────────────────────────────────

class ConfigDialog(QDialog):
    """Modal dialog for editing the three Coding Plan API keys.

    Keys are masked by default (Password echo); a checkbox reveals them.
    On accept, ``get_keys()`` returns the three entered values (empty
    string means "clear this vendor's key"). The caller persists them via
    each fetcher's ``set_key`` and triggers a refresh.
    """

    # (config key, display label, placeholder)
    _FIELDS = (
        ("deepseek", "DeepSeek", "sk-..."),
        ("glm", "GLM 智谱AI", "xxxxxxxx.xxxxxxxxxxxxxxxx"),
        ("minimax", "MiniMax", "sk-cp-..."),
    )

    def __init__(self, deepseek_key="", glm_key="", minimax_key="", parent=None):
        super().__init__(parent)
        self.setWindowTitle("API Key 配置")
        self.setModal(True)
        self.setMinimumWidth(460)
        # The parent (MainWidget) is a WA_TranslucentBackground window, and that
        # flag propagates to child top-level windows like this dialog. On a
        # transparent window the QLineEdit text/echo dots render invisible
        # (white-on-white, since the palette falls back to a light text colour).
        # Force the dialog opaque so it gets a proper, paintable background.
        self.setAttribute(Qt.WA_TranslucentBackground, False)
        self.setStyleSheet("""
            QDialog { background: #F4F7FB; }
            QLineEdit {
                padding: 6px 8px;
                border: 1px solid rgba(0,0,0,0.12);
                border-radius: 6px;
                background: white;
                color: #1E2433;
                selection-background-color: #4F87FF;
                selection-color: white;
                font-size: 12px;
            }
            QLineEdit:focus { border: 1px solid #4F87FF; }
            QCheckBox { spacing: 6px; color: #586070; font-size: 11px; }
        """)

        root = QVBoxLayout(self)
        root.setContentsMargins(20, 20, 20, 16)
        root.setSpacing(12)

        hint = QLabel("配置各厂商 Coding Plan 的 API Key。留空则清除该厂商的已保存配置。")
        hint.setWordWrap(True)
        hint.setStyleSheet("color: #586070; font-size: 11px;")
        root.addWidget(hint)

        initial = {"deepseek": deepseek_key, "glm": glm_key, "minimax": minimax_key}
        self._edits = {}
        for key, name, placeholder in self._FIELDS:
            row = QHBoxLayout()
            row.setSpacing(8)
            lbl = QLabel(name)
            lbl.setFixedWidth(90)
            lbl.setStyleSheet("color: #1E2433; font-size: 12px; font-weight: 600;")
            edit = QLineEdit()
            edit.setPlaceholderText(placeholder)
            edit.setEchoMode(QLineEdit.Password)
            edit.setText(initial.get(key, ""))
            row.addWidget(lbl)
            row.addWidget(edit, 1)
            wrap = QWidget()
            wrap.setLayout(row)
            root.addWidget(wrap)
            self._edits[key] = edit

        # Show / hide keys
        self._show_cb = QCheckBox("显示 Key")
        self._show_cb.toggled.connect(self._toggle_echo)
        root.addWidget(self._show_cb)

        root.addStretch()

        # Buttons
        btn_row = QHBoxLayout()
        btn_row.addStretch()
        cancel_btn = QPushButton("取消")
        cancel_btn.setStyleSheet("""
            QPushButton {
                padding: 6px 16px; border-radius: 6px;
                border: 1px solid rgba(0,0,0,0.1);
                background: white; color: #1E2433; font-size: 12px;
            }
            QPushButton:hover { background: #EEF3FB; }
        """)
        cancel_btn.clicked.connect(self.reject)
        save_btn = QPushButton("保存并刷新")
        save_btn.setDefault(True)
        save_btn.setStyleSheet("""
            QPushButton {
                padding: 6px 16px; border-radius: 6px; border: none;
                background: #2563EB; color: white;
                font-size: 12px; font-weight: 600;
            }
            QPushButton:hover { background: #1D4ED8; }
        """)
        save_btn.clicked.connect(self.accept)
        btn_row.addWidget(cancel_btn)
        btn_row.addWidget(save_btn)
        root.addLayout(btn_row)

    def _toggle_echo(self, on: bool):
        mode = QLineEdit.Normal if on else QLineEdit.Password
        for edit in self._edits.values():
            edit.setEchoMode(mode)

    def get_keys(self) -> dict:
        """Return {deepseek, glm, minimax} → stripped text (empty = cleared)."""
        return {key: edit.text().strip() for key, edit in self._edits.items()}


# ── Main Floating Widget ─────────────────────────────────────────────────────

class MainWidget(QWidget):
    """Frameless, always-on-top floating widget."""

    # Emitted to ask the background FetchWorker for a refresh. Connected to
    # FetchWorker.request_fetch (lives on another thread) → queued safely.
    fetch_requested = Signal()

    def __init__(self, data_watcher: DataWatcher, config_path: str = None,
                 deepseek_fetcher=None, glm_fetcher=None, minimax_fetcher=None):
        super().__init__()
        self._data_watcher = data_watcher
        self._config_path = config_path
        self._deepseek_fetcher = deepseek_fetcher
        self._glm_fetcher = glm_fetcher
        self._minimax_fetcher = minimax_fetcher
        self._resizing = False
        self._resize_edge = None
        self._pinned = True

        # Window flags (pinned by default)
        self._set_pin_flags(True)

        # WA_TranslucentBackground must be set BEFORE the native window is
        # created (i.e. before show()). It makes the top-level HWND truly
        # transparent so rounded corners and the DWM backdrop show through.
        self.setAttribute(Qt.WA_TranslucentBackground)

        # DWM acrylic/mica will be applied in showEvent() *after* the native
        # window is fully realised — calling winId() here can return a stale
        # handle on some builds and leave a white frame behind.
        self._acrylic_ok = False

        # Card opacity: will be adjusted after acrylic probe in showEvent().
        self._bg = BackgroundWidget()
        self._bg.set_bg_color(QColor(221, 228, 239, 253))

        # Size — fixed so all three GLM labels always fit without scrolling.
        self.setFixedSize(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT)

        # Anchor to the bottom-right of the work area (floating above the
        # taskbar with a gap, instead of the OS-default window position).
        self._anchor_to_bottom_right()

        # ── Layout ──
        # IMPORTANT: zero margins on the master layout. Any transparent margin
        # between the OS window edge and the BackgroundWidget card appears as a
        # white/light rectangle on Windows because DWM composites the alpha
        # channel against the desktop colour — that is the "white frame" bug.
        master_layout = QVBoxLayout(self)
        master_layout.setContentsMargins(0, 0, 0, 0)
        master_layout.setSpacing(0)

        # Background container layout
        bg_layout = QVBoxLayout(self._bg)
        bg_layout.setContentsMargins(0, 0, 0, 0)
        bg_layout.setSpacing(0)

        # Title bar
        self._titlebar = TitleBar()
        self._titlebar.close_clicked.connect(self._on_close)
        self._titlebar.minimize_clicked.connect(self._on_minimize)
        self._titlebar.pin_clicked.connect(self._toggle_pin)
        self._titlebar.config_clicked.connect(self._open_config)
        self._titlebar.set_pinned(self._pinned)
        bg_layout.addWidget(self._titlebar)

        # Separator
        sep = QFrame()
        sep.setFixedHeight(1)
        sep.setStyleSheet("background: rgba(0,0,0,0.08);")
        bg_layout.addWidget(sep)

        # Scroll area for service cards
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarAlwaysOff)
        scroll.setVerticalScrollBarPolicy(Qt.ScrollBarAsNeeded)
        scroll.setStyleSheet("""
            QScrollArea {
                border: none;
                background: transparent;
            }
            QScrollBar:vertical {
                width: 4px;
                background: transparent;
                margin: 2px 0;
            }
            QScrollBar::handle:vertical {
                background: rgba(0,0,0,0.18);
                border-radius: 2px;
                min-height: 20px;
            }
            QScrollBar::handle:vertical:hover {
                background: rgba(0,0,0,0.32);
            }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
                height: 0;
            }
        """)

        # Content widget inside scroll area
        content_widget = QWidget()
        content_widget.setStyleSheet("background: transparent;")
        self._content_layout = QVBoxLayout(content_widget)
        self._content_layout.setContentsMargins(12, 8, 12, 8)
        self._content_layout.setSpacing(8)
        self._content_layout.addStretch()  # Pushes cards to top

        scroll.setWidget(content_widget)
        bg_layout.addWidget(scroll, 1)

        # Footer
        self._footer = QLabel()
        self._footer.setStyleSheet("color: #9AA1AD; font-size: 10px; padding: 6px 14px;")
        self._footer.setAlignment(Qt.AlignCenter)
        bg_layout.addWidget(self._footer)

        master_layout.addWidget(self._bg)

        # ── System Tray ──
        self._setup_tray()

        # ── Data binding ──
        self._data_watcher.data_changed.connect(self._on_data_updated)

        # ── Background API fetcher ──
        # All three API calls run on a worker thread (in parallel) so the
        # network I/O never blocks the GUI event loop. The window renders
        # from balance.json immediately and updates once data arrives.
        self._fetch_worker = None
        self._fetch_thread = None
        self._setup_fetch_worker()

        # Initial render from the local file (instant); kick off a background
        # fetch to overlay live API data shortly after.
        # Render the file skeleton as N/A first; the live numbers arrive
        # from the background fetch shortly. Showing the stale file values
        # as if they were live reads as fake/mock data, which is jarring.
        initial = self._data_watcher.load()
        for svc in initial.services:
            svc.loading = True
        self._render(initial)
        self.request_refresh()

        # Timer: refresh footer every 30s
        self._footer_timer = QTimer(self)
        self._footer_timer.timeout.connect(self._update_footer)
        self._footer_timer.start(30000)

        # Keyboard shortcut: F5 to refresh
        refresh_shortcut = QShortcut(QKeySequence(Qt.Key_F5), self)
        refresh_shortcut.activated.connect(self._manual_refresh)

    # ── Window Placement ──────────────────────────────────────────────────────

    def _anchor_to_bottom_right(self):
        """Move to the bottom-right of the primary screen's work area.

        ``availableGeometry`` already excludes the taskbar, so aligning the
        window's bottom-right corner to it (minus WINDOW_SCREEN_MARGIN) places
        the widget just above the taskbar with a visible gap on the right and
        bottom edges.
        """
        screen = QApplication.primaryScreen()
        if not screen:
            return
        geo = screen.availableGeometry()
        m = WINDOW_SCREEN_MARGIN
        x = geo.x() + geo.width() - self.width() - m
        y = geo.y() + geo.height() - self.height() - m
        # Clamp so a too-tall window never slides off the top-left of the work area.
        self.move(max(geo.x(), x), max(geo.y(), y))

    # ── Show / Resize Events ──────────────────────────────────────────────────

    def showEvent(self, event):
        """Apply acrylic/mica backdrop + clip window to rounded rect.

        Called when the window is first shown and after pin toggle (hide→show
        recreates the native HWND, losing all DWM attributes and region).
        """
        super().showEvent(event)
        if sys.platform == "win32":
            hwnd = int(self.winId())
            if not self._acrylic_ok:
                self._acrylic_ok = _try_enable_acrylic(hwnd)
                if self._acrylic_ok:
                    self._bg.set_bg_color(QColor(221, 228, 239, 252))
                else:
                    self._bg.set_bg_color(QColor(221, 228, 239, 255))
            self._apply_topmost()

    def resizeEvent(self, event):
        """No-op — kept to satisfy Qt expectations."""
        super().resizeEvent(event)

    # ── System Tray ──────────────────────────────────────────────────────────

    @staticmethod
    def _make_tray_icon(color: str = "#00C853") -> QIcon:
        """Create a simple colored dot icon for the system tray."""
        pixmap = QPixmap(16, 16)
        pixmap.fill(Qt.transparent)
        painter = QPainter(pixmap)
        painter.setRenderHint(QPainter.Antialiasing)
        painter.setBrush(QColor(color))
        painter.setPen(Qt.NoPen)
        painter.drawRoundedRect(2, 2, 12, 12, 4, 4)
        painter.end()
        return QIcon(pixmap)

    def _setup_tray(self):
        self._tray = QSystemTrayIcon(self)
        # Use custom icon file (fallback to programmatic dot)
        icon_path = resource_path("aistatus.ico")
        if os.path.isfile(icon_path):
            self._tray.setIcon(QIcon(icon_path))
        else:
            self._tray.setIcon(self._make_tray_icon("#00C853"))
        self._tray.setToolTip("AI API 余额监控")

        menu = QMenu()
        show_action = QAction("显示", self)
        show_action.triggered.connect(self._show_window)
        menu.addAction(show_action)

        menu.addSeparator()

        quit_action = QAction("退出", self)
        quit_action.triggered.connect(self._on_quit)
        menu.addAction(quit_action)

        self._tray.setContextMenu(menu)
        self._tray.activated.connect(self._on_tray_activated)
        self._tray.show()

    def _on_tray_activated(self, reason):
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self._show_window()

    def _show_window(self):
        self.show()
        self.raise_()
        self.activateWindow()

    def _on_close(self):
        self.hide()

    def _on_minimize(self):
        self.hide()

    def _on_quit(self):
        self._footer_timer.stop()
        self._stop_fetch_worker()
        QApplication.quit()

    # ── Pin / Always-on-Top ─────────────────────────────────────────────────

    def _set_pin_flags(self, pinned: bool):
        """Apply window flags for pinned/unpinned state."""
        flags = Qt.FramelessWindowHint | Qt.Tool
        if pinned:
            flags |= Qt.WindowStaysOnTopHint
        self.setWindowFlags(flags)

    def _apply_topmost(self):
        """Force topmost z-order via Win32 SetWindowPos.

        Qt.WindowStaysOnTopHint alone is unreliable on a WA_TranslucentBackground
        + DWM-acrylic frameless window — the same class of Qt-hint-vs-Win32 gap
        as the acrylic backdrop problem. SetWindowPos with HWND_TOPMOST /
        HWND_NOTOPMOST pins z-order directly. Called from showEvent so it runs
        on first show and after every pin toggle (hide→show recreates the HWND).
        """
        if sys.platform != "win32":
            return
        hwnd = int(self.winId())
        if not hwnd:
            return
        from ctypes import wintypes
        user32 = ctypes.windll.user32
        user32.SetWindowPos.argtypes = [
            wintypes.HWND, wintypes.HWND,
            ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
            ctypes.c_uint,
        ]
        user32.SetWindowPos.restype = wintypes.BOOL
        SWP_NOMOVE = 0x0002
        SWP_NOSIZE = 0x0001
        SWP_NOACTIVATE = 0x0010
        insert_after = wintypes.HWND(-1) if self._pinned else wintypes.HWND(-2)
        flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
        user32.SetWindowPos(hwnd, insert_after, 0, 0, 0, 0, flags)

    def _toggle_pin(self):
        """Toggle always-on-top pin state.

        setWindowFlags() recreates the native window, which discards both
        WA_TranslucentBackground and any DWM backdrop attributes — so we must
        re-apply them after rebuilding flags, or the white frame returns.
        Reset _acrylic_ok so showEvent() will re-apply the backdrop.
        """
        self._pinned = not self._pinned
        visible = self.isVisible()
        self.hide()
        self._set_pin_flags(self._pinned)
        self.setAttribute(Qt.WA_TranslucentBackground)
        # Reset so showEvent() will probe and re-apply the DWM effect
        self._acrylic_ok = False
        self._titlebar.set_pinned(self._pinned)
        if visible:
            self.show()
            self.raise_()
            self.activateWindow()

    # ── Background Fetch ─────────────────────────────────────────────────────

    def _setup_fetch_worker(self):
        """Create the background fetch worker on a dedicated QThread.

        The worker owns the fetchers (moved to its thread). The GUI asks
        for refreshes via ``fetch_requested``; results come back on
        ``fetched``, marshalled onto the GUI thread by Qt.
        """
        self._fetch_worker = FetchWorker(
            deepseek_fetcher=self._deepseek_fetcher,
            glm_fetcher=self._glm_fetcher,
            minimax_fetcher=self._minimax_fetcher,
        )
        self._fetch_thread = QThread(self)
        self._fetch_worker.moveToThread(self._fetch_thread)
        self.fetch_requested.connect(self._fetch_worker.request_fetch)
        self._fetch_worker.fetched.connect(self._on_fetch_complete)
        self._fetch_thread.start()

    def _stop_fetch_worker(self):
        """Stop the background fetch thread cleanly on exit."""
        thread = self._fetch_thread
        if thread is not None:
            thread.quit()
            thread.wait(2000)

    def request_refresh(self):
        """Trigger a background refresh of all API data (non-blocking).

        Safe to call from the GUI thread at any time; the worker drops
        re-entrant requests if a fetch is already in flight.
        """
        self.fetch_requested.emit()

    def _on_fetch_complete(self, ds_data, glm_data, mm_data):
        """Merge freshly-fetched API data into a fresh file load + re-render.

        Runs on the GUI thread (Qt marshals the signal across threads).
        The file is re-read here so edits to balance.json made between the
        fetch firing and landing are reflected.
        """
        data = self._data_watcher.load()
        self._data_watcher.merge_api_into(data, ds_data, glm_data, mm_data)
        self._render(data)

    # ── Data Rendering ───────────────────────────────────────────────────────

    def refresh(self, data: BalanceData):
        """Programmatic refresh with an externally-supplied BalanceData."""
        self._render(data)

    def _on_data_updated(self, data: BalanceData):
        """Called when balance.json changes on disk.

        Re-renders immediately from the (fast) local read, then kicks off a
        background API fetch so the live numbers are re-overlaid.
        """
        self._render(data)
        self.request_refresh()

    def _render(self, data: BalanceData):
        # Title
        self._titlebar.set_title(data.title)

        # Update tray icon color based on data health (only for programmatic icon)
        icon_path = resource_path("aistatus.ico")
        if not os.path.isfile(icon_path):
            tray_color = "#00C853"  # green = healthy
            for svc in data.services:
                if svc.type == "quota":
                    for item in svc.items:
                        if item.percentage >= 85:
                            tray_color = "#FF5252"  # red = nearly full
                            break
                elif svc.type == "balance" and svc.balance <= 10:
                    tray_color = "#FF5252"  # red = low balance
                if tray_color == "#FF5252":
                    break
            self._tray.setIcon(self._make_tray_icon(tray_color))

        layout = self._content_layout

        # Snapshot existing widgets (cards + placeholder) so we can reuse
        # matching cards by service name instead of destroying and
        # recreating the whole stack on every refresh — that's what caused
        # the flicker on each refresh.
        current_widgets = []
        for i in range(layout.count()):
            w = layout.itemAt(i).widget()
            if w is not None:
                current_widgets.append(w)
        existing_cards = {
            w._service.name: w for w in current_widgets if isinstance(w, ServiceCard)
        }

        # Build the ordered list of widgets the refreshed view needs.
        wanted = []
        if data.services:
            for svc in data.services:
                card = existing_cards.pop(svc.name, None)
                if card is not None:
                    card.update_service(svc)   # reuse — no flicker
                else:
                    card = ServiceCard(svc)    # new service → new card
                wanted.append(card)
        else:
            wanted.append(self._make_empty_placeholder())

        # Detach every widget from the layout (the trailing stretch has no
        # widget and stays put), drop leftovers, then re-insert in order
        # before the stretch.
        for w in current_widgets:
            layout.removeWidget(w)
        wanted_ids = {id(w) for w in wanted}
        for w in current_widgets:
            if id(w) not in wanted_ids:
                w.deleteLater()
        for idx, w in enumerate(wanted):
            layout.insertWidget(idx, w)

        # Footer
        self._update_footer(data)

    def _update_footer(self, data: BalanceData = None):
        if data is None:
            data = self._data_watcher.data
        if data:
            self._footer.setText(f"⏱ 更新于 {data.update_time_str}")
        else:
            self._footer.setText("")

    def _make_empty_placeholder(self) -> QLabel:
        """The 'no data' label shown when balance.json has no services."""
        empty = QLabel("暂无数据\n\n请编辑 balance.json 添加服务")
        empty.setStyleSheet("color: #586070; font-size: 12px; padding: 40px;")
        empty.setAlignment(Qt.AlignCenter)
        return empty

    # ── Resize ───────────────────────────────────────────────────────────────

    def mousePressEvent(self, event: QMouseEvent):
        if event.button() == Qt.LeftButton:
            pos = event.position().toPoint()
            edge = self._get_resize_edge(pos)
            if edge:
                self._resizing = True
                self._resize_edge = edge
                self._resize_start_pos = event.globalPosition().toPoint()
                self._resize_start_size = self.size()
                self.setCursor(self._resize_cursor(edge))
                event.accept()
                return
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event: QMouseEvent):
        if self._resizing and self._resize_edge:
            delta = event.globalPosition().toPoint() - self._resize_start_pos
            new_w = self._resize_start_size.width()
            new_h = self._resize_start_size.height()

            edge = self._resize_edge
            if "right" in edge:
                new_w = max(WINDOW_MIN_WIDTH, self._resize_start_size.width() + delta.x())
            if "bottom" in edge:
                new_h = max(WINDOW_MIN_HEIGHT, self._resize_start_size.height() + delta.y())

            self.resize(new_w, new_h)
            event.accept()
            return

        # Update cursor on edges
        if not self._resizing:
            pos = event.position().toPoint()
            edge = self._get_resize_edge(pos)
            self.setCursor(self._resize_cursor(edge) if edge else Qt.ArrowCursor)

        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event: QMouseEvent):
        if event.button() == Qt.LeftButton and self._resizing:
            self._resizing = False
            self._resize_edge = None
            self.setCursor(Qt.ArrowCursor)
            event.accept()
            return
        super().mouseReleaseEvent(event)

    def _get_resize_edge(self, pos: QPoint):
        """Check if pos is on a resize edge."""
        w, h = self.width(), self.height()
        margin = 6
        edges = []
        if pos.x() >= w - margin and pos.x() <= w:
            edges.append("right")
        if pos.y() >= h - margin and pos.y() <= h:
            edges.append("bottom")
        return "_".join(edges) if edges else None

    # ── Context Menu & Shortcuts ──────────────────────────────────────────

    def contextMenuEvent(self, event):
        menu = QMenu(self)
        menu.setStyleSheet("""
            QMenu {
                background: #2a2a3a;
                color: #ddd;
                border: 1px solid rgba(255,255,255,0.1);
                border-radius: 8px;
                padding: 4px;
                font-size: 12px;
            }
            QMenu::item {
                padding: 6px 20px;
                border-radius: 4px;
            }
            QMenu::item:hover {
                background: rgba(255,255,255,0.1);
                color: white;
            }
            QMenu::separator {
                height: 1px;
                background: rgba(255,255,255,0.06);
                margin: 4px 8px;
            }
        """)

        refresh_action = QAction("刷新数据", self)
        refresh_action.triggered.connect(self._manual_refresh)
        menu.addAction(refresh_action)

        edit_action = QAction("编辑数据文件", self)
        edit_action.triggered.connect(self._open_data_file)
        menu.addAction(edit_action)

        menu.addSeparator()

        pin_label = "取消置顶" if self._pinned else "置顶窗口"
        pin_action = QAction(pin_label, self)
        pin_action.triggered.connect(self._toggle_pin)
        menu.addAction(pin_action)

        menu.addSeparator()

        show_action = QAction("显示窗口", self)
        show_action.triggered.connect(self._show_window)
        menu.addAction(show_action)

        menu.addSeparator()

        quit_action = QAction("退出", self)
        quit_action.triggered.connect(self._on_quit)
        menu.addAction(quit_action)

        menu.exec(event.globalPos())

    def _manual_refresh(self):
        """Force a background refresh of all API data."""
        self.request_refresh()

    def _open_config(self):
        """Open the API-key config dialog; persist + refresh on save.

        Pre-fills each field with the fetcher's current key so the user can
        view/edit without re-entering everything. An empty field clears that
        vendor's saved key. On save, keys are persisted (config.json) via
        each fetcher's ``set_key`` and a background refresh is triggered so
        the cards reflect the new keys immediately.
        """
        dlg = ConfigDialog(
            deepseek_key=self._fetcher_key(self._deepseek_fetcher),
            glm_key=self._fetcher_key(self._glm_fetcher),
            minimax_key=self._fetcher_key(self._minimax_fetcher),
            parent=self,
        )
        if dlg.exec() != QDialog.Accepted:
            return
        keys = dlg.get_keys()
        for fetcher, value in (
            (self._deepseek_fetcher, keys["deepseek"]),
            (self._glm_fetcher, keys["glm"]),
            (self._minimax_fetcher, keys["minimax"]),
        ):
            if fetcher is not None:
                fetcher.set_key(value)
        self.request_refresh()

    def prompt_for_keys_if_needed(self):
        """Pop the API-key dialog on first run when no vendor key is set.

        When none of the three fetchers has a key (fresh install, no
        config.json yet), open the config dialog once at startup so the user
        is guided to enter their keys instead of facing empty cards.
        """
        fetchers = [self._deepseek_fetcher, self._glm_fetcher, self._minimax_fetcher]
        if not any(getattr(f, "has_key", False) for f in fetchers if f is not None):
            self._open_config()

    @staticmethod
    def _fetcher_key(fetcher) -> str:
        return fetcher.api_key if fetcher is not None else ""

    def _open_data_file(self):
        """Open balance.json in default (Notepad) editor."""
        import subprocess
        path = getattr(self._data_watcher, '_file_path', None)
        if path:
            subprocess.Popen(["notepad", path], shell=True)

    def _resize_cursor(self, edge: str):
        if edge == "right":
            return Qt.SizeHorCursor
        elif edge == "bottom":
            return Qt.SizeVerCursor
        elif edge == "right_bottom":
            return Qt.SizeFDiagCursor
        return Qt.ArrowCursor
