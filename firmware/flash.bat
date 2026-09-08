@echo off
REM VORAX ESP32-S3 Flash & Monitor Automation Script (Windows)

setlocal

set PORT=%1
if "%PORT%"=="" set PORT=COM3

set BAUD=%2
if "%BAUD%"=="" set BAUD=921600

echo ========================================================
echo   VORAX ESP32-S3 ONE-CLICK FLASH SCRIPT
echo ========================================================
echo Target Port: %PORT%
echo Target Baud: %BAUD%
echo.

idf.py -p %PORT% -b %BAUD% flash monitor

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Flashing failed! Ensure ESP-IDF is sourced and device is in download mode.
    exit /b %ERRORLEVEL%
)
