# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller spec for the AI API balance monitor.

Build:  pyinstaller aistatus.spec --noconfirm
Output: dist/aistatus/aistatus.exe  (onedir)

Notes
-----
* ``--onedir`` (COLLECT) is used instead of ``--onefile``: a PySide6 tray app
  starts far faster from a directory, and writable user data (config.json /
  balance.json) then lives beside the exe where the user can find and edit it.
* Read-only resources (icons/, aistatus.ico) are bundled via ``datas`` and
  resolved at runtime through ``paths.resource_path`` (sys._MEIPASS).
* balance.json is bundled as a first-run template; ``paths.ensure_user_data``
  copies it next to the exe on first launch if it is missing.
"""

block_cipher = None

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('icons', 'icons'),
        ('aistatus.ico', '.'),
        ('balance.json', '.'),
    ],
    hiddenimports=[],
    hookspath=[],
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='aistatus',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,          # windowed: no console window
    disable_windowed_traceback=False,
    icon='aistatus.ico',
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name='aistatus',
)
