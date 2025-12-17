@echo off
setlocal EnableDelayedExpansion

REM Set UTF-8 code page for proper Unicode character display
chcp 65001 >nul 2>&1

REM ==============================================================================
REM  🎬 ORBIT E2E DEMO WITH VIDEO RECORDING (Windows)
REM  Scenario: Deep Space Telemetry Ingestion + Screen Capture
REM  Version: 2.2.0-alpha
REM  Purpose: Record demonstration for marketing, training, documentation
REM ==============================================================================
REM
REM  TECHNICAL NOTES - Windows Batch Input Handling:
REM  ===============================================
REM  This script uses PowerShell Read-Host for user input. See demo-orbit.bat
REM  for detailed technical explanation of why native batch commands (pause,
REM  timeout, choice) fail with background processes and how PowerShell solves it.
REM
REM  Key points:
REM  - Background processes (cargo, npm, ffmpeg) use '< nul' stdin isolation
REM  - PowerShell Read-Host for reliable user input
REM  - UTF-8 encoding via 'chcp 65001'
REM ==============================================================================

set "ORBIT_ROOT=%CD%"
set "DEMO_SOURCE=%TEMP%\orbit_demo_source_%RANDOM%"
set "DEMO_DEST=%TEMP%\orbit_demo_dest_%RANDOM%"
set "API_URL=http://localhost:8080"
set "DASHBOARD_URL=http://localhost:5173"
set "VIDEO_DIR=%ORBIT_ROOT%\demo-recordings"

REM Create timestamp
for /f "tokens=2 delims==" %%a in ('wmic OS Get localdatetime /value') do set "dt=%%a"
set "TIMESTAMP=%dt:~0,4%%dt:~4,2%%dt:~6,2%-%dt:~8,2%%dt:~10,2%%dt:~12,2%"
set "VIDEO_FILE=%VIDEO_DIR%\orbit-demo-%TIMESTAMP%.mp4"

echo ╔════════════════════════════════════════════╗
echo ║    🎬 ORBIT DEMO ORCHESTRATOR + RECORDER  ║
echo ║     Scenario: Deep Space Telemetry Sync    ║
echo ╚════════════════════════════════════════════╝
echo.

REM Check for recording tools
set "RECORDER="
where ffmpeg >nul 2>nul
if %errorlevel% EQU 0 (
    set "RECORDER=ffmpeg"
    echo ✓ Found ffmpeg for screen recording
) else (
    echo ⚠ WARNING: ffmpeg not found
    echo.
    echo To enable video recording, install ffmpeg:
    echo   1. Download from https://ffmpeg.org/download.html
    echo   2. Extract and add to PATH
    echo   3. Or use: choco install ffmpeg
    echo.
    set /p "CONTINUE=Continue without recording? [y/N]: "
    if /i not "!CONTINUE!"=="y" exit /b 1
)

REM Create video directory
if not exist "%VIDEO_DIR%" mkdir "%VIDEO_DIR%"

REM Recording state
set "RECORDING_PID="

goto :Main

:StartRecording
if "%RECORDER%"=="" goto :EOF
echo 🎥 Starting screen recording...

REM Get screen resolution
for /f "tokens=2 delims==" %%a in ('wmic path Win32_VideoController get CurrentHorizontalResolution /value ^| find "="') do set "WIDTH=%%a"
for /f "tokens=2 delims==" %%a in ('wmic path Win32_VideoController get CurrentVerticalResolution /value ^| find "="') do set "HEIGHT=%%a"

REM Start ffmpeg recording (GDI capture)
start /B "Orbit-Recording" ffmpeg -f gdigrab -framerate 30 -i desktop -vcodec libx264 -preset ultrafast -pix_fmt yuv420p "%VIDEO_FILE%" >nul 2>nul

REM Get PID (approximate)
timeout /t 1 /nobreak >nul
for /f "tokens=2" %%a in ('tasklist ^| findstr "ffmpeg"') do set "RECORDING_PID=%%a"

if not "%RECORDING_PID%"=="" (
    echo ✓ Recording started
    echo   Output: %VIDEO_FILE%
) else (
    echo ⚠ Failed to start recording
)
goto :EOF

:StopRecording
if "%RECORDING_PID%"=="" goto :EOF
echo 🎬 Stopping recording...

REM Send Ctrl+C to ffmpeg (graceful stop)
taskkill /PID %RECORDING_PID% /T >nul 2>nul

REM Wait for file to be written
timeout /t 3 /nobreak >nul

if exist "%VIDEO_FILE%" (
    for %%A in ("%VIDEO_FILE%") do set "FILE_SIZE=%%~zA"
    set /a "SIZE_MB=!FILE_SIZE! / 1048576"
    echo ✓ Recording saved: %VIDEO_FILE% (!SIZE_MB! MB)

    REM Generate thumbnail
    where ffmpeg >nul 2>nul
    if %errorlevel% EQU 0 (
        set "THUMB_FILE=%VIDEO_FILE:~0,-4%.jpg"
        ffmpeg -i "%VIDEO_FILE%" -ss 00:00:05 -vframes 1 "!THUMB_FILE!" >nul 2>nul
        if exist "!THUMB_FILE!" (
            echo   Thumbnail: !THUMB_FILE!
        )
    )
) else (
    echo ⚠ Recording file not found
)
goto :EOF

:Main

REM 1. Pre-flight Checks
echo.
echo [1/6] Initiating Pre-flight Systems Check...

where cargo >nul 2>nul
if %errorlevel% NEQ 0 (
    echo ❌ Critical Error: Cargo not found
    goto :Error
)
echo ✓ Found Cargo

where npm >nul 2>nul
if %errorlevel% NEQ 0 (
    echo ❌ Critical Error: NPM not found
    goto :Error
)
echo ✓ Found NPM

where curl >nul 2>nul
if %errorlevel% NEQ 0 (
    echo ❌ Critical Error: curl not found
    goto :Error
)
echo ✓ Found curl

REM 2. Data Fabrication
echo.
echo [2/6] Fabricating Synthetic Telemetry Data...

if exist "%DEMO_SOURCE%" rmdir /s /q "%DEMO_SOURCE%" >nul 2>nul
if exist "%DEMO_DEST%" rmdir /s /q "%DEMO_DEST%" >nul 2>nul
mkdir "%DEMO_SOURCE%"
mkdir "%DEMO_DEST%"

echo    Generating binary blobs...
fsutil file createnew "%DEMO_SOURCE%\telemetry_alpha.bin" 52428800 >nul 2>nul
fsutil file createnew "%DEMO_SOURCE%\telemetry_beta.bin" 20971520 >nul 2>nul
fsutil file createnew "%DEMO_SOURCE%\telemetry_gamma.bin" 104857600 >nul 2>nul

echo    Generating flight logs...
for /L %%i in (1,1,20) do (
    echo %date% %time% [TELEMETRY] Sensor %%i: Temp !RANDOM! C > "%DEMO_SOURCE%\flight_log_%%i.log"
)

REM Create manifest
(
echo {
echo   "mission_id": "DEMO_RECORDING_%RANDOM%",
echo   "timestamp": "%date% %time%",
echo   "recording": true
echo }
) > "%DEMO_SOURCE%\mission_manifest.json"

echo ✓ Created synthetic dataset at %DEMO_SOURCE%

REM 3. System Ignition
echo.
echo [3/6] Igniting Orbit Core Systems...

set ORBIT_JWT_SECRET=demo-secret-key-must-be-32-chars-long

echo    → Launching backend...
cd "%ORBIT_ROOT%\crates\orbit-web"
start /B "Orbit-Server" cmd /c "cargo run --quiet --bin orbit-server > %ORBIT_ROOT%\orbit-server.log 2>&1"
cd "%ORBIT_ROOT%"

echo    → Launching dashboard...
cd "%ORBIT_ROOT%\dashboard"
start /B "Orbit-Dashboard" cmd /c "npm run dev -- --host 0.0.0.0 > %ORBIT_ROOT%\orbit-dashboard.log 2>&1"
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
        echo ❌ Timeout waiting for API
        goto :Error
    )
    set /a MOD=!RETRY_COUNT! %% 10
    if !MOD! EQU 0 echo|set /p="."
    goto :HealthCheckLoop
)

echo.
echo ✓ Control Plane is Online.

timeout /t 2 /nobreak >nul

REM Open browser
start "%DASHBOARD_URL%"

REM 4. Ready for Recording
echo.
echo ╔════════════════════════════════════════════════════════════╗
echo ║                  READY FOR LAUNCH                          ║
echo ╚════════════════════════════════════════════════════════════╝
echo.
echo    Dashboard: %DASHBOARD_URL%
echo.
echo Arrange your windows so the dashboard is visible.
echo.
pause

REM Start recording
call :StartRecording

REM Give user time to focus
echo.
echo Recording will start in 3 seconds...
timeout /t 3 /nobreak >nul

REM 5. Job Injection
echo.
echo [4/6] Injecting Job via Magnetar API...

set "JSON_SOURCE=%DEMO_SOURCE:\=\\%"
set "JSON_DEST=%DEMO_DEST:\=\\%"

echo    → Creating job...
for /f "delims=" %%i in ('curl -s -X POST "%API_URL%/api/create_job" -H "Content-Type: application/json" -d "{\"source\": \"%JSON_SOURCE%\", \"destination\": \"%JSON_DEST%\", \"compress\": true, \"verify\": true, \"parallel_workers\": 4}"') do set "JOB_ID=%%i"

echo %JOB_ID%| findstr /R "^[0-9][0-9]*$" >nul
if %errorlevel% EQU 0 (
    echo ✓ Job Created! Job ID: %JOB_ID%

    echo    → Starting job...
    for /f "delims=" %%i in ('curl -s -X POST "%API_URL%/api/run_job" -H "Content-Type: application/json" -d "{\"job_id\": %JOB_ID%}"') do set "RUN_RESPONSE=%%i"
    echo ✓ Job Started!
) else (
    echo ❌ Failed to create job
    goto :Error
)

REM 6. Observation Phase
echo.
echo [5/6] Observation Phase (Recording)...
echo.
echo ═══════════════════════════════════════════════════════════
echo   🎥 RECORDING IN PROGRESS
echo   Watch the dashboard and narrate as needed.
echo.
echo   • Demonstrate the Visual Chunk Map
echo   • Show the Live Telemetry graphs
echo   • Highlight real-time progress tracking
echo ═══════════════════════════════════════════════════════════
echo.
pause

REM Stop recording
call :StopRecording

echo.
echo ✓ Demo complete! Video saved.

REM Cleanup
goto :Cleanup

:Error
set "EXIT_CODE=1"
goto :Cleanup

:Cleanup
echo.
echo [6/6] Initiating Orbital Decay (Cleanup)...

REM Stop recording if still running
call :StopRecording

echo    → Killing processes...
taskkill /FI "WINDOWTITLE eq Orbit-Server*" /T /F >nul 2>nul
taskkill /FI "WINDOWTITLE eq Orbit-Dashboard*" /T /F >nul 2>nul
timeout /t 1 /nobreak >nul
taskkill /F /IM cargo.exe >nul 2>nul
taskkill /F /IM orbit-server.exe >nul 2>nul
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":5173"') do (
    taskkill /F /PID %%a >nul 2>nul
)

echo    → Removing test data...
if exist "%DEMO_SOURCE%" rmdir /s /q "%DEMO_SOURCE%" >nul 2>nul
if exist "%DEMO_DEST%" rmdir /s /q "%DEMO_DEST%" >nul 2>nul

echo ✓ Systems Offline. Data purged.

if "%RECORDER%" NEQ "" if exist "%VIDEO_FILE%" (
    echo.
    echo ════════════════════════════════════════
    echo 📹 Demo recording available at:
    echo    %VIDEO_FILE%
    echo ════════════════════════════════════════
)

if defined EXIT_CODE (
    exit /b %EXIT_CODE%
) else (
    exit /b 0
)
