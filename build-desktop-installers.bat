@echo off
echo ===================================================
echo  Yntra Platform - Desktop Installer Compilation
echo ===================================================

echo [1/3] Building release desktop binary (yntra-ui)...
cargo build --release -p yntra-ui
if %ERRORLEVEL% neq 0 (
    echo Error: Release compilation failed.
    exit /b %ERRORLEVEL%
)

echo [2/3] Checking for NSIS compiler (makensis.exe)...
where makensis >nul 2>nul
if %ERRORLEVEL% eq 0 (
    echo Compiling NSIS installer...
    makensis packaging\windows\installer.nsi
    if %ERRORLEVEL% neq 0 (
        echo Error: NSIS compilation failed.
        exit /b %ERRORLEVEL%
    )
    echo Installer generated at target\release\YntraPlatform-Setup.exe
) else (
    echo Warning: makensis.exe was not found in PATH. Skipping installer compilation.
    echo Binary available at target\release\yntra-ui.exe
)

echo [3/3] Executing EV Code Signing script...
powershell -ExecutionPolicy Bypass -File packaging\windows\sign-installer.ps1

echo ===================================================
echo  SUCCESS: Desktop release build process complete!
echo ===================================================
