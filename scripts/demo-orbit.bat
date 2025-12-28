@echo off
setlocal EnableDelayedExpansion

REM Set UTF-8 code page for proper Unicode character display
chcp 65001 >nul 2>&1

REM ==============================================================================
REM  🛰️  ORBIT E2E DEMONSTRATION HARNESS
REM  Scenario: Deep Space Telemetry Ingestion
REM  Version: 2.2.0-alpha
REM ==============================================================================
REM
REM  TECHNICAL NOTES - Windows Batch Input Handling:
REM  ===============================================
REM  This script uses PowerShell Read-Host for user input instead of native
REM  batch commands (pause, timeout, choice) because:
REM
REM  1. Background Process stdin Conflict:
REM     - cargo.exe and node.exe run via 'start /B' (background)
REM     - These processes share stdin with parent batch script
REM     - Native batch input commands fail when stdin is contested
REM
REM  2. Observed Issues with Native Commands:
REM     - 'pause' hangs, Ctrl+C doesn't work
REM     - 'timeout' shows key codes instead of continuing
REM     - 'choice' corrupts: "'tinue' is not recognized"
REM     - Input intercepted by wrong process or corrupted
REM
REM  3. PowerShell Read-Host Solution:
REM     - Spawns dedicated PowerShell process for input
REM     - No stdin sharing with background processes
REM     - Ctrl+C properly propagates
REM     - Reliable across Command Prompt, PowerShell, Windows Terminal
REM
REM  4. Background Process stdin Isolation:
REM     - Use '< nul' redirection to disconnect stdin:
REM       start /B "Name" cmd /c "cargo run < nul > log 2>&1"
REM     - Prevents background process from stealing keyboard input
REM
REM  See DEMO_GUIDE.md for troubleshooting and more details.
REM ==============================================================================

REM Configuration
set "ORBIT_ROOT=%CD%"
set "DEMO_SOURCE=%TEMP%\orbit_demo_source_%RANDOM%"
set "DEMO_DEST=%TEMP%\orbit_demo_dest_%RANDOM%"
set "API_URL=http://localhost:8080"
set "DASHBOARD_URL=http://localhost:5173"

REM Visuals (ANSI colors for Windows 10+)
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

cls
echo.
echo %BLUE%╔════════════════════════════════════════════╗%RESET%
echo %BLUE%║       🛰️  ORBIT DEMO ORCHESTRATOR         ║%RESET%
echo %BLUE%║     Scenario: Deep Space Telemetry Sync    ║%RESET%
echo %BLUE%╚════════════════════════════════════════════╝%RESET%
echo.

REM 1. Pre-flight Checks
echo %YELLOW%[1/6] Initiating Pre-flight Systems Check...%RESET%

where cargo >nul 2>nul
if %errorlevel% NEQ 0 (
    echo %RED%❌ Critical Error: Rust/Cargo not found.%RESET%
    goto :Cleanup
)
echo %GREEN%✓ Found Cargo%RESET%

where npm >nul 2>nul
if %errorlevel% NEQ 0 (
    echo %RED%❌ Critical Error: Node/NPM not found.%RESET%
    goto :Cleanup
)
echo %GREEN%✓ Found NPM%RESET%

where curl >nul 2>nul
if %errorlevel% NEQ 0 (
    echo %RED%❌ Critical Error: curl not found.%RESET%
    goto :Cleanup
)
echo %GREEN%✓ Found curl%RESET%

REM Check if ports are available
netstat -ano | findstr ":8080" >nul 2>nul
if %errorlevel% EQU 0 (
    echo %YELLOW%⚠ Warning: Port 8080 may be in use%RESET%
)

netstat -ano | findstr ":5173" >nul 2>nul
if %errorlevel% EQU 0 (
    echo %YELLOW%⚠ Warning: Port 5173 may be in use%RESET%
)

REM 2. Data Fabrication
echo.
echo %YELLOW%[2/6] Fabricating Synthetic Telemetry Data...%RESET%

if exist "%DEMO_SOURCE%" rmdir /s /q "%DEMO_SOURCE%" >nul 2>nul
if exist "%DEMO_DEST%" rmdir /s /q "%DEMO_DEST%" >nul 2>nul
mkdir "%DEMO_SOURCE%"
mkdir "%DEMO_DEST%"

echo    Generating binary blobs...
REM Create dummy files using fsutil (requires admin) or fallback method
fsutil file createnew "%DEMO_SOURCE%\telemetry_alpha.bin" 52428800 >nul 2>nul
if %errorlevel% NEQ 0 (
    REM Fallback: create files using PowerShell
    powershell -Command "$content = New-Object byte[] 52428800; (New-Object Random).NextBytes($content); [IO.File]::WriteAllBytes('%DEMO_SOURCE%\telemetry_alpha.bin', $content)" >nul 2>nul
)

fsutil file createnew "%DEMO_SOURCE%\telemetry_beta.bin" 20971520 >nul 2>nul
if %errorlevel% NEQ 0 (
    powershell -Command "$content = New-Object byte[] 20971520; (New-Object Random).NextBytes($content); [IO.File]::WriteAllBytes('%DEMO_SOURCE%\telemetry_beta.bin', $content)" >nul 2>nul
)

fsutil file createnew "%DEMO_SOURCE%\telemetry_gamma.bin" 104857600 >nul 2>nul
if %errorlevel% NEQ 0 (
    powershell -Command "$content = New-Object byte[] 104857600; (New-Object Random).NextBytes($content); [IO.File]::WriteAllBytes('%DEMO_SOURCE%\telemetry_gamma.bin', $content)" >nul 2>nul
)

echo    Generating flight logs...
for /L %%i in (1,1,20) do (
    echo %date% %time% [TELEMETRY] Sensor reading #%%i: Temperature !RANDOM! C, Radiation !RANDOM! mSv > "%DEMO_SOURCE%\flight_log_%%i.log"
)

REM Create manifest
(
echo {
echo   "mission_id": "DEEP_SPACE_001",
echo   "timestamp": "%date% %time%",
echo   "telescope": "Hubble-Successor",
echo   "data_type": "telemetry",
echo   "total_files": 23,
echo   "estimated_size_mb": 170
echo }
) > "%DEMO_SOURCE%\mission_manifest.json"

echo %GREEN%✓ Created synthetic dataset at %DEMO_SOURCE%%RESET%

REM Count files
set "FILE_COUNT=0"
for %%f in ("%DEMO_SOURCE%\*") do set /a FILE_COUNT+=1
echo    Total files: %FILE_COUNT%

REM 3. System Ignition
echo.
echo %YELLOW%[3/6] Igniting Orbit Core Systems...%RESET%

REM Set Dev Secret
set ORBIT_JWT_SECRET=demo-secret-key-must-be-32-chars-long

echo    → Launching Control Plane (Magnetar)...
cd "%ORBIT_ROOT%\crates\orbit-web"
start /B "Orbit-Server" cmd /c "cargo run --quiet --bin orbit-server < nul > %ORBIT_ROOT%\orbit-server.log 2>&1"
cd "%ORBIT_ROOT%"

echo    → Launching Dashboard...
cd "%ORBIT_ROOT%\dashboard"
start /B "Orbit-Dashboard" cmd /c "npm run dev -- --host 0.0.0.0 < nul > %ORBIT_ROOT%\orbit-dashboard.log 2>&1"
cd "%ORBIT_ROOT%"

echo    → Waiting for API stability...
set "RETRY_COUNT=0"
set "MAX_RETRIES=60"

:HealthCheckLoop
timeout /t 1 /nobreak >nul
curl -s -f "%API_URL%/api/health" >nul 2>nul
if %errorlevel% NEQ 0 (
    set /a RETRY_COUNT+=1
    if !RETRY_COUNT! GEQ %MAX_RETRIES% (
        echo.
        echo %RED%❌ Timeout waiting for API to become healthy!%RESET%
        echo %RED%Check logs:%RESET%
        echo   - Server: %ORBIT_ROOT%\orbit-server.log
        echo   - Dashboard: %ORBIT_ROOT%\orbit-dashboard.log
        goto :Cleanup
    )
    echo|set /p="."
    goto :HealthCheckLoop
)

echo.
echo %GREEN%✓ Control Plane is Online.%RESET%

REM Brief pause for dashboard
timeout /t 2 /nobreak >nul

REM 4. User Interaction & Scenario Execution
echo.
echo %BLUE%╔════════════════════════════════════════════════════════════╗%RESET%
echo %BLUE%║                  READY FOR LAUNCH                          ║%RESET%
echo %BLUE%╚════════════════════════════════════════════════════════════╝%RESET%
echo.
echo    %BOLD%Dashboard:%RESET% %CYAN%%DASHBOARD_URL%%RESET%
echo    %BOLD%API Docs:%RESET%  %CYAN%%API_URL%/swagger-ui%RESET%
echo.
echo %CYAN%Please open your browser to the Dashboard URL above.%RESET%
echo %CYAN%Explore the interface, then return here to continue...%RESET%
echo.
echo %BOLD%════════════════════════════════════════════════════════════%RESET%
echo %GREEN%  Press ENTER to continue to job creation...%RESET%
echo %YELLOW%  (Ctrl+C to abort)%RESET%
echo %BOLD%════════════════════════════════════════════════════════════%RESET%
echo.
powershell -Command "$null = Read-Host 'Press ENTER to continue'"
echo.
echo %GREEN%✓ Continuing to job creation...%RESET%
echo.
echo %YELLOW%[4/6] Injecting Job via Magnetar API...%RESET%

REM Construct JSON payload (Windows path escaping)
set "JSON_SOURCE=%DEMO_SOURCE:\=\\%"
set "JSON_DEST=%DEMO_DEST:\=\\%"

echo    → Creating job...

REM Create job
for /f "delims=" %%i in ('curl -s -X POST "%API_URL%/api/create_job" -H "Content-Type: application/json" -d "{\"source\": \"%JSON_SOURCE%\", \"destination\": \"%JSON_DEST%\", \"compress\": true, \"verify\": true, \"parallel_workers\": 4}"') do set "JOB_ID=%%i"

REM Verify job ID is numeric
echo %JOB_ID%| findstr /R "^[0-9][0-9]*$" >nul
if %errorlevel% EQU 0 (
    echo %GREEN%✓ Job Created! Job ID: %JOB_ID%%RESET%

    echo    → Starting job execution...
    for /f "delims=" %%i in ('curl -s -X POST "%API_URL%/api/run_job" -H "Content-Type: application/json" -d "{\"job_id\": %JOB_ID%}"') do set "RUN_RESPONSE=%%i"

    echo %GREEN%✓ Job Started! Response: %RUN_RESPONSE%%RESET%
) else (
    echo %RED%❌ Failed to create job. Response: %JOB_ID%%RESET%
    goto :Cleanup
)

REM 5. Observation Phase
echo.
echo %YELLOW%[5/6] Observation Phase...%RESET%
echo.
echo %BOLD%══════════════════════════════════════════════════════════%RESET%
echo   %CYAN%Watch the dashboard for live updates:%RESET%
echo   • %GREEN%Visual Chunk Map%RESET% - Real-time transfer progress
echo   • %GREEN%Live Telemetry%RESET% - Transfer speed and statistics
echo   • %GREEN%Job Status%RESET% - Current state of the transfer
echo.
echo   %BOLD%Processing Details:%RESET%
echo   • Source: %DEMO_SOURCE%
echo   • Destination: %DEMO_DEST%
echo   • Data Volume: ~170 MB
echo   • Verification: Enabled (checksum validation)
echo   • Compression: Enabled
echo   • Parallel Workers: 4
echo %BOLD%══════════════════════════════════════════════════════════%RESET%
echo.
echo %CYAN%Check the dashboard to see the transfer progress and Visual Chunk Map!%RESET%
echo.
echo %BOLD%════════════════════════════════════════════════════════════%RESET%
echo %GREEN%  Press ENTER to cleanup and exit...%RESET%
echo %YELLOW%  (Ctrl+C to abort)%RESET%
echo %BOLD%════════════════════════════════════════════════════════════%RESET%
echo.
powershell -Command "$null = Read-Host 'Press ENTER to cleanup'"
echo.
echo %GREEN%✓ Starting cleanup...%RESET%
echo.

REM 6. Cleanup
:Cleanup
echo.
echo %YELLOW%[6/6] Initiating Orbital Decay (Cleanup)...%RESET%

echo    → Killing Orbit processes...
taskkill /FI "WINDOWTITLE eq Orbit-Server*" /T /F >nul 2>nul
taskkill /FI "WINDOWTITLE eq Orbit-Dashboard*" /T /F >nul 2>nul

REM Fallback: kill by process name
timeout /t 1 /nobreak >nul
taskkill /F /IM cargo.exe >nul 2>nul
taskkill /F /IM orbit-server.exe >nul 2>nul

REM Kill node processes on port 5173
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":5173"') do (
    taskkill /F /PID %%a >nul 2>nul
)

echo    → Removing synthetic data...
if exist "%DEMO_SOURCE%" rmdir /s /q "%DEMO_SOURCE%" >nul 2>nul
if exist "%DEMO_DEST%" rmdir /s /q "%DEMO_DEST%" >nul 2>nul

echo.
echo %GREEN%✓ Systems Offline. Data purged.%RESET%
echo.
echo %GREEN%Demo complete! Thank you for experiencing Orbit.%RESET%
echo.
timeout /t 3 /nobreak >nul
exit /b 0
