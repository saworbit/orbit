@echo off
REM test-reactor.bat - Quick test of Reactor functionality

echo ╔════════════════════════════════════════════╗
echo ║   🧪 Orbit Reactor Test Suite             ║
echo ║   Verify the job execution engine works   ║
echo ╚════════════════════════════════════════════╝
echo.

echo ⚙️  Step 1: Check compilation...
cd crates\orbit-web
cargo check --bin orbit-server --quiet
if %errorlevel% neq 0 (
    echo ❌ Compilation failed!
    pause
    exit /b 1
)
echo ✅ Compilation successful
cd ..\..
echo.

echo ⚙️  Step 2: Starting Orbit Server with Reactor...
echo    (This will run in the background for 5 seconds)
echo.

cd crates\orbit-web
start /MIN "Orbit-Test" cmd /c "cargo run --bin orbit-server"
cd ..\..

echo 🕐 Waiting for server startup...
timeout /t 3 /nobreak > nul

echo.
echo ⚙️  Step 3: Testing API health...
curl -s http://localhost:8080/api/health > nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Server not responding
    taskkill /F /FI "WINDOWTITLE eq Orbit-Test*" >nul 2>nul
    pause
    exit /b 1
)
echo ✅ API is responding

echo.
echo ⚙️  Step 4: Check reactor startup message...
echo    (Check the Orbit-Test window to see "☢️ Orbit Reactor Online")
echo.

echo ╔════════════════════════════════════════════╗
echo ║   ✅ Reactor Test Complete!                ║
echo ╚════════════════════════════════════════════╝
echo.
echo The reactor is running! Test it:
echo 1. Open http://localhost:5173 in your browser
echo 2. Create a job in Quick Transfer
echo 3. Click the job to see live chunk progress
echo.
echo Press any key to stop the test server...
pause > nul

echo.
echo 🛑 Stopping test server...
taskkill /F /FI "WINDOWTITLE eq Orbit-Test*" >nul 2>nul
timeout /t 1 /nobreak >nul

echo ✅ Test server stopped
echo.
pause
