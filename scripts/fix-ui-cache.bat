@echo off
REM fix-ui-cache.bat - Nuclear option for cache issues

echo ╔════════════════════════════════════════════╗
echo ║   🔥 UI Cache Buster                       ║
echo ║   Fix "still looks like old one" issues    ║
echo ╚════════════════════════════════════════════╝
echo.

echo Step 1: Kill any running dev servers...
taskkill /F /IM node.exe >nul 2>nul
taskkill /F /FI "WINDOWTITLE eq Orbit-Dashboard-Process*" >nul 2>nul
echo   ✓ Cleared running processes
echo.

echo Step 2: Clear Vite cache...
if exist "dashboard\node_modules\.vite" (
    rmdir /s /q "dashboard\node_modules\.vite"
    echo   ✓ Deleted .vite cache
) else (
    echo   ✓ No .vite cache found
)
echo.

echo Step 3: Clear production build...
if exist "dashboard\dist" (
    rmdir /s /q "dashboard\dist"
    echo   ✓ Deleted dist folder
) else (
    echo   ✓ No dist folder found
)
echo.

echo Step 4: Verify JobDetail.tsx has real API...
findstr /C:"useQuery" dashboard\src\components\jobs\JobDetail.tsx >nul
if %errorlevel% equ 0 (
    echo   ✅ JobDetail.tsx has useQuery (correct version!)
) else (
    echo   ❌ ERROR: JobDetail.tsx missing useQuery!
    echo   Something went wrong. Check the file.
    pause
    exit /b 1
)
echo.

echo Step 5: Starting FRESH dev server...
echo   (This will open in a new window)
echo.
cd dashboard
start "Orbit-Fresh-UI" cmd /k "npm run dev"
cd ..

timeout /t 3 /nobreak > nul

echo.
echo ╔════════════════════════════════════════════╗
echo ║   ✅ Fresh start complete!                 ║
echo ╚════════════════════════════════════════════╝
echo.
echo Now do this:
echo   1. Open http://localhost:5173 in an INCOGNITO/PRIVATE window
echo   2. Hard refresh: Ctrl+Shift+R (or Ctrl+F5)
echo   3. Login: admin / orbit2025
echo   4. Go to Jobs page
echo   5. Click ANY job in the list
echo   6. You should see the NEW chunk map!
echo.
echo If you still see the old UI:
echo   - Make sure you're on http://localhost:5173 (NOT :8080)
echo   - Close ALL browser tabs and restart browser
echo   - Check that backend is running (separate terminal)
echo.

pause
