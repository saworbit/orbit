@echo off
setlocal

echo.
echo  ====================================
echo   Orbit Nebula - Demo Jobs Creator
echo  ====================================
echo.

set "DB_PATH=%~dp0data\magnetar.db"

:: Check if sqlite3 is available
where sqlite3 >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [!] sqlite3 not found in PATH
    echo.
    echo This script requires sqlite3 to create demo jobs.
    echo.
    echo Options:
    echo   1. Install SQLite from https://sqlite.org/download.html
    echo   2. Use create-demo-jobs-api.bat instead [requires server running]
    echo.
    goto :end
)

:: Check if data directory exists
if not exist "%~dp0data" mkdir "%~dp0data"

echo [*] Creating demo jobs in: %DB_PATH%
echo.

:: Create SQL file for table creation
echo CREATE TABLE IF NOT EXISTS jobs ^( > "%TEMP%\orbit_demo.sql"
echo     id INTEGER PRIMARY KEY AUTOINCREMENT, >> "%TEMP%\orbit_demo.sql"
echo     source TEXT NOT NULL, >> "%TEMP%\orbit_demo.sql"
echo     destination TEXT NOT NULL, >> "%TEMP%\orbit_demo.sql"
echo     compress BOOLEAN NOT NULL DEFAULT 0, >> "%TEMP%\orbit_demo.sql"
echo     verify BOOLEAN NOT NULL DEFAULT 0, >> "%TEMP%\orbit_demo.sql"
echo     parallel INTEGER, >> "%TEMP%\orbit_demo.sql"
echo     status TEXT NOT NULL DEFAULT 'pending', >> "%TEMP%\orbit_demo.sql"
echo     progress REAL NOT NULL DEFAULT 0.0, >> "%TEMP%\orbit_demo.sql"
echo     total_chunks INTEGER NOT NULL DEFAULT 0, >> "%TEMP%\orbit_demo.sql"
echo     completed_chunks INTEGER NOT NULL DEFAULT 0, >> "%TEMP%\orbit_demo.sql"
echo     failed_chunks INTEGER NOT NULL DEFAULT 0, >> "%TEMP%\orbit_demo.sql"
echo     created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP, >> "%TEMP%\orbit_demo.sql"
echo     updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP >> "%TEMP%\orbit_demo.sql"
echo ^); >> "%TEMP%\orbit_demo.sql"

sqlite3 "%DB_PATH%" < "%TEMP%\orbit_demo.sql"
if %ERRORLEVEL% NEQ 0 (
    echo [!] Failed to create database schema
    goto :cleanup
)

echo [+] Database schema ready
echo.
echo [*] Inserting demo jobs...
echo.

:: Clear existing jobs
sqlite3 "%DB_PATH%" "DELETE FROM jobs;"

:: Insert demo jobs using simple INSERT statements
sqlite3 "%DB_PATH%" "INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks) VALUES ('C:/Users/Documents/Projects', 'S3://backup-bucket/projects', 'completed', 100.0, 150, 150, 0);"
echo     [+] Job 1: Completed backup - Projects to S3

sqlite3 "%DB_PATH%" "INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks) VALUES ('C:/Data/Archive/2024', '//NAS01/Backups/Archive', 'running', 67.5, 400, 270, 0);"
echo     [+] Job 2: Running transfer - Archive to NAS

sqlite3 "%DB_PATH%" "INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks) VALUES ('C:/Media/Photos', 'S3://media-archive/photos/2024', 'pending', 0.0, 0, 0, 0);"
echo     [+] Job 3: Pending sync - Photos to S3

sqlite3 "%DB_PATH%" "INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks) VALUES ('C:/Temp/ImportantFiles', '//SERVER02/SharedDrive/Import', 'failed', 23.5, 80, 18, 2);"
echo     [+] Job 4: Failed transfer - ImportantFiles to Server

sqlite3 "%DB_PATH%" "INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks) VALUES ('D:/Databases/MySQL/backups', 'S3://db-backups/mysql/daily', 'running', 89.2, 50, 44, 1);"
echo     [+] Job 5: Running backup - MySQL to S3

sqlite3 "%DB_PATH%" "INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks) VALUES ('C:/Config/AppSettings', '//BACKUP01/Configs', 'completed', 100.0, 12, 12, 0);"
echo     [+] Job 6: Completed config backup

echo.
echo  ====================================
echo   Demo Jobs Created Successfully!
echo  ====================================
echo.
echo   Summary:
echo     - 2 Completed jobs
echo     - 2 Running jobs
echo     - 1 Pending job
echo     - 1 Failed job
echo.
echo   Start the Nebula server to see them:
echo     start-nebula.bat
echo.
echo   Then open: http://localhost:8080
echo.

:cleanup
if exist "%TEMP%\orbit_demo.sql" del "%TEMP%\orbit_demo.sql"

:end
endlocal
pause
