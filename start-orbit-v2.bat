@echo off
REM start-orbit-v2.bat - Orbit V2.2.0 Development Environment (Windows)

setlocal

echo ╔════════════════════════════════════════════╗
echo ║   🚀 Orbit V2.2.0 Development Launcher    ║
echo ║   The Separation: Control Plane + Dashboard ║
echo ╚════════════════════════════════════════════╝
echo.

REM Start the Control Plane (Backend)
echo 🧠 Starting Orbit Control Plane (Rust API)...
echo    → Directory: crates\orbit-web
echo    → Endpoint: http://localhost:8080
echo    → Swagger UI: http://localhost:8080/swagger-ui
echo.

start "Orbit Control Plane" /D "%~dp0crates\orbit-web" cmd /k "cargo run --bin orbit-server"

REM Wait a moment for the server to start
timeout /t 3 /nobreak > nul

REM Start the Dashboard (Frontend)
echo.
echo 🎨 Starting Orbit Dashboard (React SPA)...
echo    → Directory: dashboard
echo    → Dev Server: http://localhost:5173
echo    → HMR: Enabled (Vite)
echo.

start "Orbit Dashboard" /D "%~dp0dashboard" cmd /k "npm run dev"

echo.
echo ╔════════════════════════════════════════════╗
echo ║        ✓ Orbit V2.2.0 is Running!         ║
echo ╚════════════════════════════════════════════╝
echo.
echo 📋 Access Points:
echo    Dashboard:    http://localhost:5173
echo    API:          http://localhost:8080/api
echo    API Docs:     http://localhost:8080/swagger-ui
echo.
echo 💡 Tips:
echo    • Dashboard has hot reload enabled
echo    • API changes require cargo rebuild
echo    • Close both terminal windows to stop services
echo.
echo Press any key to exit this launcher window...
pause > nul
