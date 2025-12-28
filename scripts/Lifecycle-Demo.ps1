<#
.SYNOPSIS
    Orbit Data Lifecycle Demonstration v3.0 - Windows
.DESCRIPTION
    A guided architectural tour of Orbit's capabilities:
    Generation -> Observation -> Replication -> Verification -> Mutation -> Sync.
.NOTES
    Documentation: docs/guides/TESTING_SCRIPTS_GUIDE.md
#>

$ErrorActionPreference = "Stop"

# --- Configuration ---
$BaseDir = Get-Location
$WorkDir = Join-Path $BaseDir "orbit_lifecycle_lab"
$SrcDir = Join-Path $WorkDir "sector_alpha"
$DstDir = Join-Path $WorkDir "sector_beta"
$BinaryPath = Join-Path $BaseDir "target\release\orbit.exe"
$LogFile = Join-Path $WorkDir "mission_log.txt"
$RequiredSpaceMB = 500

# --- Visuals ---
function Log-Info($Msg) { Write-Host "[ORBIT SYSTEM] $Msg" -ForegroundColor Cyan }
function Log-Success($Msg) { Write-Host "[SUCCESS] $Msg" -ForegroundColor Green }
function Log-Warn($Msg) { Write-Host "[ATTENTION] $Msg" -ForegroundColor Yellow }
function Log-Error($Msg) { Write-Host "[CRITICAL] $Msg" -ForegroundColor Red }

function Request-Observation($Title, $Instruction) {
    Write-Host "`n>>> OBSERVATION POINT: $Title" -ForegroundColor Yellow
    Write-Host ">>> Action: $Instruction" -ForegroundColor Gray
    Write-Host "--> Press any key to confirm and proceed..." -ForegroundColor DarkYellow
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    Write-Host ""
}

# --- Execution Block ---
try {
    # 1. Setup & Safety
    if (Test-Path $WorkDir) { Remove-Item $WorkDir -Recurse -Force }
    New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

    Log-Info "Initializing Lifecycle Protocol..."

    $Drive = Get-PSDrive -Name $WorkDir.Substring(0,1)
    if (($Drive.Free / 1MB) -lt $RequiredSpaceMB) {
        throw "Insufficient disk space."
    }

    if (-not (Test-Path $BinaryPath)) {
        Log-Info "Compiling Orbit binary..."
        cargo build --release
        if (-not (Test-Path $BinaryPath)) { throw "Compilation failed." }
    }

    # 2. Topology Generation
    Log-Info "Constructing Data Topology..."
    $Dirs = @("logs\archive", "images\raw", "db\shards")
    foreach ($d in $Dirs) { New-Item -ItemType Directory -Path (Join-Path $SrcDir $d) -Force | Out-Null }

    # Create Text Data
    "Cluster Config v1" | Set-Content (Join-Path $SrcDir "config.json")
    1..5 | ForEach-Object { "Log Entry $_" | Set-Content (Join-Path $SrcDir "logs\archive\log_$_.txt") }

    # Create Binary Data (5MB)
    Log-Info "Synthesizing binary payloads..."
    $buffer = New-Object byte[] (5MB)
    [System.Random]::new().NextBytes($buffer)
    [System.IO.File]::WriteAllBytes((Join-Path $SrcDir "images\raw\texture.bin"), $buffer)

    Log-Success "Dataset Generated."

    # 3. Observation 1
    Request-Observation "Source Topology" "Open File Explorer to '$SrcDir'. Observe the folder structure."

    # 4. Replication
    Log-Info "Engaging Replication Engine (COPY)..."
    $timer = [System.Diagnostics.Stopwatch]::StartNew()

    $p = Start-Process -FilePath $BinaryPath -ArgumentList "-s", """$SrcDir""", "-d", """$DstDir""", "-R", "-m", "copy" -PassThru -NoNewWindow -Wait -RedirectStandardOutput $LogFile

    $timer.Stop()
    if ($p.ExitCode -ne 0) { throw "Copy failed." }
    Log-Success "Transfer complete in $($timer.Elapsed.TotalMilliseconds) ms."

    # 5. Observation 2
    Request-Observation "Replication Check" "Inspect '$DstDir'. Verify files match Source."

    # 6. Integrity Audit
    Log-Info "Performing Cryptographic Audit (SHA256)..."

    function Get-TreeHash($Root) {
        Get-ChildItem $Root -Recurse -File | ForEach-Object {
            @{ Path = $_.FullName.Substring($Root.Length); Hash = (Get-FileHash $_.FullName).Hash }
        } | Sort-Object Path
    }

    $SrcHashes = Get-TreeHash $SrcDir
    $DstHashes = Get-TreeHash $DstDir

    if (Compare-Object $SrcHashes $DstHashes -Property Path, Hash) {
        throw "Integrity Mismatch Detected!"
    }
    Log-Success "AUDIT PASSED: 100% Data Consistency."

    # 7. Mutation & Sync
    Log-Info "Simulating Data Drift (Mutation)..."
    Remove-Item (Join-Path $SrcDir "logs\archive\log_1.txt")
    "New Drift Data" | Set-Content (Join-Path $SrcDir "drift_file.dat")

    Request-Observation "Drift Analysis" "Check '$SrcDir'. Note that 'log_1.txt' is deleted and 'drift_file.dat' is new."

    Log-Info "Engaging Synchronization Engine (SYNC)..."
    $p = Start-Process -FilePath $BinaryPath -ArgumentList "-s", """$SrcDir""", "-d", """$DstDir""", "-R", "-m", "sync" -PassThru -NoNewWindow -Wait
    if ($p.ExitCode -ne 0) { throw "Sync failed." }
    Log-Success "Sync operation complete."

    # 8. Final Observation
    Request-Observation "Convergence" "Check '$DstDir'. The Destination should now mirror the Modified Source."

} catch {
    Log-Error $_
    exit 1
} finally {
    Log-Info "Decommissioning Lab..."
    if (Test-Path $WorkDir) { Remove-Item $WorkDir -Recurse -Force }
    Log-Success "System Clean."
}
