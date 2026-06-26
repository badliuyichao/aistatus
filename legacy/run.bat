@echo off
chcp 65001 >nul
cd /d "%~dp0"
start /b "" pythonw main.py
echo [OK] AI API Balance Monitor started.
echo [TIP] Right-click tray icon to exit.
echo.
pause