@echo off
powershell.exe -NoProfile -STA -WindowStyle Hidden -File "%~dp0desktop.ps1"
if errorlevel 1 (
    echo ASCII Arcade could not open. Check the extracted desktop.ps1 file's Properties for an Unblock option.
    pause
)
