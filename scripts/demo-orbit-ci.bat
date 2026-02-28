@echo off
setlocal EnableDelayedExpansion

REM Set UTF-8 code page for proper Unicode character display
chcp 65001 >nul 2>&1

REM ==============================================================================
REM  🛰️  ORBIT E2E CI/CD HARNESS (Headless Mode)
REM  Scenario: Deep Space Telemetry Ingestion
REM  Version: 2.2.3-alpha
REM  Purpose: Non-interactive automated testing for CI/CD pipelines
REM ==============================================================================
REM
REM  TECHNICAL NOTES:
REM  - Background processes run via 'start /B' for non-blocking execution
REM  - Headless mode: no user input required, fully automated
REM  - UTF-8 encoding via 'chcp 65001' for proper character display
REM ==============================================================================

set "ORBIT_ROOT=%CD%"
set "DEMO_SOURCE=%TEMP%\orbit_ci_source_%RANDOM%"
set "DEMO_DEST=%TEMP%\orbit_ci_dest_%RANDOM%"
set "API_URL=http://localhost:8080"
set "METRICS_FILE=%ORBIT_ROOT%\e2e-metrics.json"
set "CURL_BIN=curl.exe"
set "CURL_BASE=%CURL_BIN% -s --show-error --connect-timeout 2 --max-time 5 --retry 0"

REM Start time (seconds since epoch approximation)
set "START_TIME=%TIME%"

echo [INFO] ╔════════════════════════════════════════════╗
echo [INFO] ║  ORBIT E2E CI/CD HARNESS (HEADLESS MODE)  ║
echo [INFO] ║   Scenario: Deep Space Telemetry Sync      ║
echo [INFO] ╚════════════════════════════════════════════╝

REM Initialize metrics
echo { > "%METRICS_FILE%"

REM 1. Pre-flight Checks
echo [INFO] [1/6] Pre-flight Systems Check...

where cargo >nul 2>nul
if %errorlevel% NEQ 0 (
    echo [ERROR] Cargo not found
    goto :Error
)
echo [SUCCESS] Found Cargo

where npm >nul 2>nul
if %errorlevel% NEQ 0 (
    echo [ERROR] NPM not found
    goto :Error
)
echo [SUCCESS] Found NPM

where curl >nul 2>nul
if %errorlevel% NEQ 0 (
    echo [ERROR] curl not found
    goto :Error
)
echo [SUCCESS] Found curl

REM 2. Data Fabrication
echo [INFO] [2/6] Fabricating Synthetic Telemetry Data...

if exist "%DEMO_SOURCE%" rmdir /s /q "%DEMO_SOURCE%" >nul 2>nul
if exist "%DEMO_DEST%" rmdir /s /q "%DEMO_DEST%" >nul 2>nul
mkdir "%DEMO_SOURCE%"
mkdir "%DEMO_DEST%"

REM Create test files (smaller for CI)
fsutil file createnew "%DEMO_SOURCE%\telemetry_alpha.bin" 10485760 >nul 2>nul
fsutil file createnew "%DEMO_SOURCE%\telemetry_beta.bin" 5242880 >nul 2>nul
fsutil file createnew "%DEMO_SOURCE%\telemetry_gamma.bin" 20971520 >nul 2>nul

for /L %%i in (1,1,10) do (
    echo %date% %time% [TELEMETRY] Sensor %%i: Temp !RANDOM! C > "%DEMO_SOURCE%\flight_log_%%i.log"
)

REM Create manifest
(
echo {
echo   "mission_id": "CI_TEST_%RANDOM%",
echo   "timestamp": "%date% %time%",
echo   "ci_mode": true
echo }
) > "%DEMO_SOURCE%\mission_manifest.json"

set "FILE_COUNT=0"
for %%f in ("%DEMO_SOURCE%\*") do set /a FILE_COUNT+=1
echo [SUCCESS] Created %FILE_COUNT% test files

REM 3. System Ignition
echo [INFO] [3/6] Igniting Orbit Core Systems...

set ORBIT_JWT_SECRET=ci-test-secret-key-must-be-32-chars

REM Start Backend
echo [INFO] Launching backend...
cd "%ORBIT_ROOT%\crates\orbit-web"

if exist "%ORBIT_ROOT%\target\release\orbit-server.exe" (
    echo [INFO] Using pre-built binary
    start /B "Orbit-Server-CI" "%ORBIT_ROOT%\target\release\orbit-server.exe" >"%ORBIT_ROOT%\orbit-server.log" 2>&1
) else (
    echo [INFO] Building from source
    start /B "Orbit-Server-CI" cargo run --quiet --bin orbit-server >"%ORBIT_ROOT%\orbit-server.log" 2>&1
)

cd "%ORBIT_ROOT%"

REM Start Frontend
echo [INFO] Launching dashboard...
cd "%ORBIT_ROOT%\dashboard"
start /B "Orbit-Dashboard-CI" npm run dev -- --host 0.0.0.0 >"%ORBIT_ROOT%\orbit-dashboard.log" 2>&1
cd "%ORBIT_ROOT%"

REM Wait for Health Check
echo [INFO] Waiting for API to become healthy...
set "RETRY_COUNT=0"
set "MAX_RETRIES=60"

:HealthCheckLoop
call :Sleep 1
%CURL_BASE% -f "%API_URL%/api/health" >nul 2>nul
if %errorlevel% NEQ 0 (
    set /a RETRY_COUNT+=1
    if !RETRY_COUNT! GEQ %MAX_RETRIES% (
        echo [ERROR] Timeout waiting for API
        type "%ORBIT_ROOT%\orbit-server.log"
        goto :Error
    )
    set /a MOD=!RETRY_COUNT! %% 10
    if !MOD! EQU 0 (
        echo [INFO] Still waiting... (!RETRY_COUNT!/%MAX_RETRIES%)
    )
    goto :HealthCheckLoop
)

echo [SUCCESS] Control Plane is online

REM Wait for dashboard
call :Sleep 3

REM 4. Job Injection
echo [INFO] [4/6] Injecting Job via Magnetar API...

REM Authenticate to get JWT token
echo [INFO] Authenticating...
set "COOKIE_JAR=%TEMP%\orbit_cookies_%RANDOM%.txt"

%CURL_BASE% -f -X POST "%API_URL%/api/auth/login" -H "Content-Type: application/json" -c "%COOKIE_JAR%" -d "{\"username\":\"admin\",\"password\":\"orbit2025\"}" > "%TEMP%\login_response.json"
findstr /C:"\"username\"" "%TEMP%\login_response.json" >nul
if %errorlevel% NEQ 0 (
    echo [ERROR] Authentication failed
    type "%TEMP%\login_response.json"
    goto :Error
)
echo [SUCCESS] Authenticated as admin

set "JSON_SOURCE=%DEMO_SOURCE:\=\\%"
set "JSON_DEST=%DEMO_DEST:\=\\%"

echo [INFO] Creating job...
for /f "delims=" %%i in ('%CURL_BASE% -f -X POST "%API_URL%/api/create_job" -H "Content-Type: application/json" -b "%COOKIE_JAR%" -d "{\"source\": \"%JSON_SOURCE%\", \"destination\": \"%JSON_DEST%\", \"compress\": true, \"verify\": true, \"parallel_workers\": 4}"') do set "JOB_ID=%%i"

echo %JOB_ID%| findstr /R "^[0-9][0-9]*$" >nul
if %errorlevel% EQU 0 (
    echo [SUCCESS] Job created: ID=%JOB_ID%

    echo [INFO] Starting job...
    for /f "delims=" %%i in ('%CURL_BASE% -f -X POST "%API_URL%/api/run_job" -H "Content-Type: application/json" -b "%COOKIE_JAR%" -d "{\"job_id\": %JOB_ID%}"') do set "RUN_RESPONSE=%%i"
    echo [SUCCESS] Job started: !RUN_RESPONSE!
) else (
    echo [ERROR] Failed to create job: %JOB_ID%
    goto :Error
)

REM 5. Monitor Job Progress
echo [INFO] [5/6] Monitoring Job Progress...

set "ELAPSED=0"
set "MAX_WAIT=120"

:MonitorLoop
call :Sleep 2
set /a ELAPSED+=2

REM Get job status (simplified - just check if destination has files)
set "DEST_FILES=0"
for %%f in ("%DEMO_DEST%\*") do set /a DEST_FILES+=1

echo [INFO] Progress check... Elapsed: !ELAPSED!s, Files: !DEST_FILES!/%FILE_COUNT%

if !DEST_FILES! EQU %FILE_COUNT% (
    echo [SUCCESS] Job completed successfully!
    goto :ValidationSuccess
)

if !ELAPSED! GEQ %MAX_WAIT% (
    echo [ERROR] Job did not complete within timeout
    goto :Error
)

goto :MonitorLoop

:ValidationSuccess
REM 6. Final Validation
echo [INFO] [6/6] Validation Complete

echo [SUCCESS] ════════════════════════════════════════
echo [SUCCESS]   E2E Demo Test: PASSED
echo [SUCCESS]   Job ID: %JOB_ID%
echo [SUCCESS]   Files Transferred: !DEST_FILES!/%FILE_COUNT%
echo [SUCCESS] ════════════════════════════════════════

REM Write metrics
(
echo   "job_id": "%JOB_ID%",
echo   "test_files_count": %FILE_COUNT%,
echo   "destination_files_count": !DEST_FILES!,
echo   "transfer_success": true,
echo   "timestamp": "%date% %time%"
echo }
) >> "%METRICS_FILE%"

REM Cleanup
goto :Cleanup

:Error
echo [ERROR] Test failed
set "EXIT_CODE=1"
goto :Cleanup

:Cleanup
echo [INFO] Cleanup initiated...

REM Kill processes
taskkill /FI "WINDOWTITLE eq Orbit-Server-CI*" /T /F >nul 2>nul
taskkill /FI "WINDOWTITLE eq Orbit-Dashboard-CI*" /T /F >nul 2>nul

call :Sleep 1
taskkill /F /IM orbit-server.exe >nul 2>nul
for /f "tokens=5" %%a in ('netstat -ano ^| findstr ":5173"') do (
    taskkill /F /PID %%a >nul 2>nul
)

REM Remove test data
if exist "%DEMO_SOURCE%" rmdir /s /q "%DEMO_SOURCE%" >nul 2>nul
if exist "%DEMO_DEST%" rmdir /s /q "%DEMO_DEST%" >nul 2>nul
if exist "%COOKIE_JAR%" del /q "%COOKIE_JAR%" >nul 2>nul
if exist "%TEMP%\login_response.json" del /q "%TEMP%\login_response.json" >nul 2>nul

echo [SUCCESS] Cleanup complete

if defined EXIT_CODE (
    exit /b %EXIT_CODE%
) else (
    exit /b 0
)

:Sleep
setlocal
set "SECONDS=%~1"
if "%SECONDS%"=="" set "SECONDS=1"
powershell -NoProfile -Command "Start-Sleep -Seconds %SECONDS%" >nul 2>&1
endlocal
exit /b 0
