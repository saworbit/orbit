@echo off
setlocal EnableDelayedExpansion

:: ==============================================================================
::  Orbit Launchpad (Windows) - v2.2.0-alpha
::  "The best way to orchestrate your data."
:: ==============================================================================

:: --- 1. Setup Colors (Windows 10/11) ---
:: Creates an ESC variable for ANSI escape codes
for /F "tokens=1,2 delims=#" %%a in ('"prompt #$H#$E# & echo on & for %%b in (1) do rem"') do (
  set "ESC=%%b"
)

set "RED=%ESC%[31m"
set "GREEN=%ESC%[32m"
set "BLUE=%ESC%[34m"
set "CYAN=%ESC%[36m"
set "YELLOW=%ESC%[33m"
set "BOLD=%ESC%[1m"
set "RESET=%ESC%[0m"

:: --- 2. ASCII Art Header ---
cls
echo.
echo %BLUE%
echo    ____      _     _ _
echo   / __ \    ^| ^|   (_) ^|
echo  ^| ^|  ^| ^|_ _^| ^|__  _^| ^|_
echo  ^| ^|  ^| ^| '__^| '_ \^| ^| __^|
echo  ^| ^|__^| ^| ^|  ^| ^|_) ^| ^| ^|_
echo   \____/^|_^|  ^|_.__/^|_^|\__^|
echo       C O N T R O L   P L A N E
echo %RESET%
echo.

:: --- 3. System Diagnostic ---
echo %BOLD%1. System Diagnostic%RESET%

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo   %RED%[FAIL] Rust is not installed.%RESET%
    goto :Error
)
echo   %GREEN%[ OK ] Rust detected%RESET%

where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo   %RED%[FAIL] Node.js is not installed.%RESET%
    goto :Error
)
echo   %GREEN%[ OK ] Node.js detected%RESET%

:: --- 4. Preparing Engines ---
echo.
echo %BOLD%2. Preparing Engines%RESET%

:: Build Rust Backend
<nul set /p "=%CYAN%  [ .. ] Compiling Control Plane...%RESET%"
cd crates\orbit-web
call cargo build --quiet --bin orbit-server > ..\..\orbit_build.log 2>&1
if %errorlevel% neq 0 (
    echo.
    echo   %RED%[FAIL] Build failed. See orbit_build.log%RESET%
    goto :Error
)
cd ..\..
echo   %ESC%[1A%ESC%[2K%GREEN%  [ OK ] Compiling Control Plane%RESET%

:: Install Node Modules if missing
if not exist "dashboard\node_modules" (
    <nul set /p "=%CYAN%  [ .. ] Installing Dashboard dependencies...%RESET%"
    cd dashboard
    call npm ci --silent > nul 2>&1
    cd ..
    echo   %ESC%[1A%ESC%[2K%GREEN%  [ OK ] Installing Dashboard dependencies%RESET%
)

:: --- 5. Ignition ---
echo.
echo %BOLD%3. Ignition%RESET%

echo %CYAN%  [ .. ] Launching Services...%RESET%

:: Start Backend (Hidden, log to file)
cd crates\orbit-web
start /B "OrbitServer" cmd /c "cargo run --quiet --bin orbit-server > ..\..\orbit_server.log 2>&1"
set SERVER_PID=RUNNING
cd ..\..

:: Start Frontend (Hidden, log to file)
cd dashboard
start /B "OrbitUI" cmd /c "npm run dev -- --clearScreen false > ..\orbit_ui.log 2>&1"
cd ..

:: Wait for API Health
:WaitLoop
timeout /t 1 /nobreak >nul
curl -s http://localhost:8080/api/health >nul 2>nul
if %errorlevel% neq 0 (
    <nul set /p "=. "
    goto :WaitLoop
)
echo.
echo   %ESC%[1A%ESC%[2K%GREEN%  [ OK ] Connection established!%RESET%

:: --- 6. Liftoff ---
echo.
echo %BOLD%========================================%RESET%
echo    %GREEN%Orbit v2.2 is ACTIVE%RESET%
echo    ------------------------------------
echo    Dashboard : %CYAN%http://localhost:5173%RESET%
echo    API Docs  : %CYAN%http://localhost:8080/swagger-ui%RESET%
echo %BOLD%========================================%RESET%
echo.

:: Open Browser
start http://localhost:5173

:: --- 7. Keep Alive & Cleanup ---
echo %YELLOW%Press any key to stop all services...%RESET%
pause >nul

echo.
echo %RED%🛑 Shutting down...%RESET%
:: Kill by image name (Forcefully)
taskkill /F /IM "orbit-server.exe" >nul 2>nul
taskkill /F /IM "node.exe" >nul 2>nul

echo %GREEN%✓ Systems offline.%RESET%
exit /b 0

:Error
echo.
echo %RED%Aborting launch.%RESET%
pause
exit /b 1
