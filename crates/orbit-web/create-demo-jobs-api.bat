@echo off
setlocal

echo.
echo  ====================================
echo   Orbit Nebula - Demo Jobs via API
echo  ====================================
echo.
echo   NOTE: The Nebula server must be running!
echo         Start it with: start-nebula.bat
echo.

set "SERVER=http://localhost:8080"

:: Check if curl is available
where curl >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [!] curl not found. Please install curl or use PowerShell.
    goto :end
)

echo [*] Logging in as admin...
curl -s -c "%TEMP%\orbit_cookies.txt" -X POST "%SERVER%/api/auth/login" -H "Content-Type: application/json" -d "{\"username\":\"admin\",\"password\":\"orbit2025\"}" > nul 2>&1

if %ERRORLEVEL% NEQ 0 (
    echo [!] Failed to login. Is the server running at %SERVER%?
    goto :end
)

echo [+] Logged in successfully
echo.
echo [*] Creating demo jobs...
echo.

:: Job 1: Large completed backup
echo     [*] Creating: Completed Projects Backup...
curl -s -b "%TEMP%\orbit_cookies.txt" -X POST "%SERVER%/api/create_job" -H "Content-Type: application/json" -d "{\"source\":\"C:/Users/Documents/Projects\",\"destination\":\"S3://backup-bucket/projects\",\"compress\":true,\"verify\":true,\"parallel_workers\":8}" > nul
echo     [+] Done

:: Job 2: Running archive transfer
echo     [*] Creating: Archive Transfer...
curl -s -b "%TEMP%\orbit_cookies.txt" -X POST "%SERVER%/api/create_job" -H "Content-Type: application/json" -d "{\"source\":\"C:/Data/Archive/2024\",\"destination\":\"//NAS01/Backups/Archive\",\"compress\":false,\"verify\":true,\"parallel_workers\":4}" > nul
echo     [+] Done

:: Job 3: Pending photo sync
echo     [*] Creating: Photo Sync...
curl -s -b "%TEMP%\orbit_cookies.txt" -X POST "%SERVER%/api/create_job" -H "Content-Type: application/json" -d "{\"source\":\"C:/Media/Photos\",\"destination\":\"S3://media-archive/photos\",\"compress\":false,\"verify\":false,\"parallel_workers\":2}" > nul
echo     [+] Done

:: Job 4: Database backup
echo     [*] Creating: MySQL Backup...
curl -s -b "%TEMP%\orbit_cookies.txt" -X POST "%SERVER%/api/create_job" -H "Content-Type: application/json" -d "{\"source\":\"D:/Databases/MySQL/backups\",\"destination\":\"S3://db-backups/mysql\",\"compress\":true,\"verify\":true,\"parallel_workers\":4}" > nul
echo     [+] Done

:: Job 5: Config backup
echo     [*] Creating: Config Backup...
curl -s -b "%TEMP%\orbit_cookies.txt" -X POST "%SERVER%/api/create_job" -H "Content-Type: application/json" -d "{\"source\":\"C:/Config/AppSettings\",\"destination\":\"//BACKUP01/Configs\",\"compress\":false,\"verify\":true,\"parallel_workers\":2}" > nul
echo     [+] Done

:: Clean up cookies
if exist "%TEMP%\orbit_cookies.txt" del "%TEMP%\orbit_cookies.txt"

echo.
echo  ====================================
echo   Demo Jobs Created!
echo  ====================================
echo.
echo   Open the dashboard to see them:
echo   %SERVER%
echo.
echo   Login: admin / orbit2025
echo.

:end
endlocal
pause
