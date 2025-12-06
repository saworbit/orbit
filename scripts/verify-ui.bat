@echo off
REM verify-ui.bat - Verify the JobDetail chunk map is working

echo ╔════════════════════════════════════════════╗
echo ║   🧪 UI Verification Checklist             ║
echo ╚════════════════════════════════════════════╝
echo.

echo ✓ Checking dashboard source files...
if exist "dashboard\src\components\jobs\JobDetail.tsx" (
    findstr /C:"useQuery" dashboard\src\components\jobs\JobDetail.tsx >nul
    if %errorlevel% equ 0 (
        echo   ✅ JobDetail.tsx has useQuery (real API)
    ) else (
        echo   ❌ JobDetail.tsx missing useQuery!
    )

    findstr /C:"ChunkMap" dashboard\src\components\jobs\JobDetail.tsx >nul
    if %errorlevel% equ 0 (
        echo   ✅ ChunkMap component present
    ) else (
        echo   ❌ ChunkMap component missing!
    )

    findstr /C:"grid-cols-20" dashboard\src\components\jobs\JobDetail.tsx >nul
    if %errorlevel% equ 0 (
        echo   ✅ Visual chunk grid configured
    ) else (
        echo   ❌ Chunk grid layout missing!
    )
) else (
    echo   ❌ JobDetail.tsx not found!
)

echo.
echo ✓ Checking production build...
if exist "dashboard\dist\index.html" (
    echo   ✅ Production build exists
    for %%I in ("dashboard\dist\assets\*.js") do (
        echo   ✅ Built: %%~nxI
        goto :found_js
    )
    :found_js
) else (
    echo   ⚠️  Production build NOT FOUND
    echo   Run: cd dashboard ^&^& npm run build
)

echo.
echo ✓ Checking backend reactor...
if exist "crates\orbit-web\src\reactor.rs" (
    echo   ✅ Reactor engine present
) else (
    echo   ❌ Reactor missing!
)

if exist "crates\orbit-web\src\progress.rs" (
    echo   ✅ Progress tracker present
) else (
    echo   ❌ Progress tracker missing!
)

echo.
echo ╔════════════════════════════════════════════╗
echo ║   📋 Development Workflow                  ║
echo ╚════════════════════════════════════════════╝
echo.
echo Option 1: Development mode (RECOMMENDED)
echo   Terminal 1:  cd crates\orbit-web ^&^& cargo run
echo   Terminal 2:  cd dashboard ^&^& npm run dev
echo   Browser:     http://localhost:5173
echo   ✅ Hot reload enabled!
echo.
echo Option 2: Production mode (embedded UI)
echo   1. cd dashboard ^&^& npm run build
echo   2. cargo run --release --features ui
echo   Browser: http://localhost:8080
echo   ⚠️  Requires rebuild after UI changes
echo.
echo ╔════════════════════════════════════════════╗
echo ║   🎯 Test the Chunk Map                    ║
echo ╚════════════════════════════════════════════╝
echo.
echo 1. Start development servers (Option 1)
echo 2. Open http://localhost:5173
echo 3. Login: admin / orbit2025
echo 4. Create a job in Quick Transfer
echo 5. Click the job in the job list
echo 6. Watch for:
echo    ✅ Green blocks filling up (completed chunks)
echo    ✅ Red blocks (failed chunks, 1%% rate)
echo    ✅ Smooth animations
echo    ✅ Progress percentage updating
echo.

pause
