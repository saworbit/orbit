@echo off
setlocal enabledelayedexpansion

:: ============================================================================
::  Orbit Nebula - Automated Startup Script (Windows)
::  Version: 1.0.0-alpha.2
:: ============================================================================

echo.
echo   ___  ____  ____ ___ _____   _   _ _____ ____  _   _ _        _
echo  / _ \^|  _ \^| __ )_ _^|_   _^| ^| \ ^| ^| ____^| __ )^| ^| ^| ^| ^|      / \
echo ^| ^| ^| ^| ^|_) ^|  _ \^| ^|  ^| ^|   ^|  \^| ^|  _^| ^|  _ \^| ^| ^| ^| ^|     / _ \
echo ^| ^|_^| ^|  _ ^<^| ^|_) ^| ^|  ^| ^|   ^| ^|\  ^| ^|___^| ^|_) ^| ^|_^| ^| ^|___ / ___ \
echo  \___/^|_^| \_\____/___^| ^|_^|   ^|_^| \_^|_____^|____/ \___/^|_____/_/   \_\
echo.
echo  Next-Gen Real-Time Web Control Center for Orbit
echo  Version: 1.0.0-alpha.2 - 100%% Backend Complete - Fully Compiling
echo.
echo ============================================================================
echo.

:: Get script directory
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%"

:: Configuration with defaults
if "%ORBIT_MAGNETAR_DB%"=="" (
    set "MAGNETAR_DB=%SCRIPT_DIR%data\magnetar.db"
) else (
    set "MAGNETAR_DB=%ORBIT_MAGNETAR_DB%"
)

if "%ORBIT_USER_DB%"=="" (
    set "USER_DB=%SCRIPT_DIR%data\users.db"
) else (
    set "USER_DB=%ORBIT_USER_DB%"
)

if "%ORBIT_JWT_SECRET%"=="" (
    set "JWT_SECRET="
) else (
    set "JWT_SECRET=%ORBIT_JWT_SECRET%"
)

if "%ORBIT_HOST%"=="" (
    set "HOST=127.0.0.1"
) else (
    set "HOST=%ORBIT_HOST%"
)

if "%ORBIT_PORT%"=="" (
    set "PORT=8080"
) else (
    set "PORT=%ORBIT_PORT%"
)

:: ============================================================================
:: Step 1: Check Prerequisites
:: ============================================================================
echo [1/7] Checking prerequisites...
echo.

:: Check for Rust
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust/Cargo not found!
    echo.
    echo Please install Rust from: https://rustup.rs/
    echo.
    pause
    exit /b 1
)

:: Get Rust version
for /f "tokens=2" %%i in ('cargo --version') do set RUST_VERSION=%%i
echo   [OK] Cargo found: %RUST_VERSION%

:: Check for wasm32-unknown-unknown target
rustup target list | findstr /C:"wasm32-unknown-unknown (installed)" >nul 2>nul
if %errorlevel% neq 0 (
    echo   [WARN] wasm32-unknown-unknown target not installed
    echo   [INFO] Installing wasm32-unknown-unknown target...
    rustup target add wasm32-unknown-unknown
    if %errorlevel% neq 0 (
        echo   [ERROR] Failed to install wasm32-unknown-unknown target
        pause
        exit /b 1
    )
    echo   [OK] wasm32-unknown-unknown target installed
) else (
    echo   [OK] wasm32-unknown-unknown target installed
)

echo.

:: ============================================================================
:: Step 2: Generate JWT Secret if Needed
:: ============================================================================
echo [2/7] Configuring JWT secret...
echo.

if "%JWT_SECRET%"=="" (
    echo   [INFO] Generating random JWT secret...

    :: Generate a random JWT secret using PowerShell
    for /f "delims=" %%i in ('powershell -Command "[Convert]::ToBase64String((1..32 | ForEach-Object { Get-Random -Minimum 0 -Maximum 256 }))"') do set JWT_SECRET=%%i

    echo   [OK] JWT secret generated
    echo.
    echo   [SECURITY WARNING]
    echo   This is a randomly generated JWT secret for development.
    echo   For production, set ORBIT_JWT_SECRET environment variable with a secure secret.
    echo.
) else (
    echo   [OK] Using provided JWT secret from ORBIT_JWT_SECRET
    echo.
)

:: ============================================================================
:: Step 3: Create Data Directories
:: ============================================================================
echo [3/7] Setting up data directories...
echo.

:: Extract directory paths
for %%i in ("%MAGNETAR_DB%") do set "MAGNETAR_DIR=%%~dpi"
for %%i in ("%USER_DB%") do set "USER_DIR=%%~dpi"

if not exist "%MAGNETAR_DIR%" (
    mkdir "%MAGNETAR_DIR%"
    echo   [OK] Created directory: %MAGNETAR_DIR%
) else (
    echo   [OK] Directory exists: %MAGNETAR_DIR%
)

if not exist "%USER_DIR%" (
    if not "%USER_DIR%"=="%MAGNETAR_DIR%" (
        mkdir "%USER_DIR%"
        echo   [OK] Created directory: %USER_DIR%
    )
) else (
    if not "%USER_DIR%"=="%MAGNETAR_DIR%" (
        echo   [OK] Directory exists: %USER_DIR%
    )
)

echo.

:: ============================================================================
:: Step 4: Set Environment Variables
:: ============================================================================
echo [4/7] Setting environment variables...
echo.

set "ORBIT_MAGNETAR_DB=%MAGNETAR_DB%"
set "ORBIT_USER_DB=%USER_DB%"
set "ORBIT_JWT_SECRET=%JWT_SECRET%"
set "ORBIT_HOST=%HOST%"
set "ORBIT_PORT=%PORT%"

echo   [OK] ORBIT_MAGNETAR_DB=%MAGNETAR_DB%
echo   [OK] ORBIT_USER_DB=%USER_DB%
echo   [OK] ORBIT_JWT_SECRET=***hidden***
echo   [OK] ORBIT_HOST=%HOST%
echo   [OK] ORBIT_PORT=%PORT%
echo.

:: ============================================================================
:: Step 5: Check if Build is Needed
:: ============================================================================
echo [5/7] Checking build status...
echo.

set "BUILD_NEEDED=false"

if not exist "target\release\orbit-web.exe" (
    echo   [INFO] No release build found - will build
    set "BUILD_NEEDED=true"
) else (
    echo   [OK] Release build exists
)

echo.

:: ============================================================================
:: Step 6: Build (if needed)
:: ============================================================================
if "%BUILD_NEEDED%"=="true" (
    echo [6/7] Building Orbit Nebula...
    echo.
    echo   This may take a few minutes on first build...
    echo.

    cargo build --release

    if %errorlevel% neq 0 (
        echo.
        echo   [ERROR] Build failed!
        echo.
        pause
        exit /b 1
    )

    echo.
    echo   [OK] Build completed successfully
    echo.
) else (
    echo [6/7] Skipping build (release binary exists)
    echo.
    echo   To force rebuild, delete target\release\orbit-web.exe or run:
    echo   cargo build --release
    echo.
)

:: ============================================================================
:: Step 7: Start the Server
:: ============================================================================
echo [7/7] Starting Orbit Nebula...
echo.
echo ============================================================================
echo  SERVER INFORMATION
echo ============================================================================
echo.
echo   Web Interface:  http://%HOST%:%PORT%
echo   Health Check:   http://%HOST%:%PORT%/health
echo   API Base:       http://%HOST%:%PORT%/api
echo.
echo   WebSocket:      ws://%HOST%:%PORT%/ws
echo.
echo   Magnetar DB:    %MAGNETAR_DB%
echo   User DB:        %USER_DB%
echo.
echo ============================================================================
echo  DEFAULT CREDENTIALS (First-Time Setup)
echo ============================================================================
echo.
echo   Username: admin
echo   Password: admin
echo.
echo   [SECURITY WARNING] Change the default admin password immediately!
echo.
echo ============================================================================
echo  API ENDPOINTS
echo ============================================================================
echo.
echo   Authentication:
echo     POST   /api/login        - Authenticate and get JWT token
echo     POST   /api/logout       - Invalidate current session
echo     GET    /api/me           - Get current user info
echo.
echo   Jobs:
echo     GET    /api/jobs         - List all jobs
echo     POST   /api/jobs         - Create new job
echo     GET    /api/jobs/:id     - Get job details
echo     DELETE /api/jobs/:id     - Delete job
echo     POST   /api/jobs/:id/run - Run job
echo.
echo   Backends:
echo     GET    /api/backends     - List configured backends
echo     GET    /api/backends/:id - Get backend details
echo.
echo ============================================================================
echo.
echo Press Ctrl+C to stop the server
echo.
echo ============================================================================
echo.

cargo run --release

if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Server failed to start!
    echo.
    pause
    exit /b 1
)

endlocal
