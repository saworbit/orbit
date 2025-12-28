<#
.SYNOPSIS
    Orbit Architecture Validation Suite v2.0 - Windows
.DESCRIPTION
    Validates build integrity, storage safety, data replication, and synchronization logic.
    Includes user observation pauses and automated cleanup.
.NOTES
    Documentation: docs/guides/TESTING_SCRIPTS_GUIDE.md
#>

$ErrorActionPreference = "Stop"

# --- Configuration ---
$BaseDir = Get-Location
$WorkDir = Join-Path $BaseDir "orbit_validation_workspace"
$SrcDir = Join-Path $WorkDir "source_data"
$DstDir = Join-Path $WorkDir "destination_data"
$BinaryPath = Join-Path $BaseDir "target\release\orbit.exe"
$LogFile = Join-Path $WorkDir "validation.log"
$RequiredSpaceMB = 500

# --- Visuals ---
function Write-Header($Text) {
    Write-Host "`n============================================================" -ForegroundColor Cyan
    Write-Host "   $Text" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
}

function Log-Info($Message) {
    $Msg = "[INFO] $Message"
    Write-Host $Msg -ForegroundColor Gray
    $Msg | Out-File $LogFile -Append -Encoding utf8
}

function Log-Success($Message) {
    $Msg = "[SUCCESS] $Message"
    Write-Host $Msg -ForegroundColor Green
    $Msg | Out-File $LogFile -Append -Encoding utf8
}

function Log-Error($Message) {
    $Msg = "[ERROR] $Message"
    Write-Host $Msg -ForegroundColor Red
    $Msg | Out-File $LogFile -Append -Encoding utf8
}

function Request-Observation($Title, $Instruction) {
    Write-Host "`n>>> OBSERVATION POINT: $Title" -ForegroundColor Yellow
    Write-Host ">>> Action: $Instruction" -ForegroundColor Yellow
    Write-Host ">>> Press any key to verify and proceed..." -ForegroundColor DarkYellow
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
}

# --- Infrastructure Checks ---
try {
    # Initialize workspace
    if (Test-Path $WorkDir) { Remove-Item $WorkDir -Recurse -Force }
    New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

    Write-Header "Phase 1: Environment & Safety Analysis"

    # 1. Disk Space Check
    $Drive = Get-PSDrive -Name $WorkDir.Substring(0,1)
    $FreeSpaceMB = [math]::Round($Drive.Free / 1MB, 2)

    if ($FreeSpaceMB -lt $RequiredSpaceMB) {
        throw "Insufficient disk space on drive $($Drive.Name). Available: $FreeSpaceMB MB. Required: $RequiredSpaceMB MB."
    }
    Log-Success "Storage Capacity Verified: $FreeSpaceMB MB available."

    # 2. Rust Toolchain
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust toolchain (cargo) is not installed or not in PATH."
    }
    Log-Success "Rust toolchain detected."

    # --- Compilation ---
    Write-Header "Phase 2: Compilation Strategy"
    Log-Info "Building Orbit release binary. Stand by..."

    $BuildParams = @{
        FilePath = "cargo"
        ArgumentList = "build", "--release"
        RedirectStandardOutput = $LogFile
        RedirectStandardError = $LogFile
        Wait = $true
        NoNewWindow = $true
    }
    Start-Process @BuildParams

    if (-not (Test-Path $BinaryPath)) {
        throw "Build artifact not found at $BinaryPath. Check logs."
    }
    Log-Success "Orbit binary compiled and verified."

    # --- Data Generation ---
    Write-Header "Phase 3: Dataset Generation"
    New-Item -ItemType Directory -Path $SrcDir -Force | Out-Null

    Log-Info "Generating shard files..."
    1..20 | ForEach-Object {
        "Data shard content signature $_" | Set-Content (Join-Path $SrcDir "shard_$_.dat")
    }

    Log-Info "Generating 15MB binary payload..."
    $buffer = New-Object byte[] (15MB)
    [System.Random]::new().NextBytes($buffer)
    [System.IO.File]::WriteAllBytes((Join-Path $SrcDir "payload.bin"), $buffer)

    Log-Success "Dataset ready."

    Request-Observation "Source Data Verification" "Open File Explorer to '$SrcDir'. Verify 20 shard files and 1 payload.bin."

    # --- Functional Test: Copy ---
    Write-Header "Phase 4: Replication Testing (Copy)"
    Log-Info "Executing Orbit Copy..."

    $CopyProcess = Start-Process -FilePath $BinaryPath -ArgumentList "-s", """$SrcDir""", "-d", """$DstDir""", "-R", "-m", "copy" -PassThru -NoNewWindow -Wait

    if ($CopyProcess.ExitCode -ne 0) { throw "Copy operation failed with exit code $($CopyProcess.ExitCode)." }
    Log-Success "Copy operation completed."

    Request-Observation "Replication Verification" "Check '$DstDir'. Ensure all files from Source are present."

    # --- Functional Test: Sync ---
    Write-Header "Phase 5: Differential Synchronization"
    Log-Info "Simulating data drift (Deleting shard_1, Adding shard_new)..."
    Remove-Item (Join-Path $SrcDir "shard_1.dat")
    "Drift content" | Set-Content (Join-Path $SrcDir "shard_new.dat")

    Log-Info "Executing Orbit Sync..."
    $SyncProcess = Start-Process -FilePath $BinaryPath -ArgumentList "-s", """$SrcDir""", "-d", """$DstDir""", "-R", "-m", "sync" -PassThru -NoNewWindow -Wait

    if ($SyncProcess.ExitCode -ne 0) { throw "Sync operation failed." }
    Log-Success "Sync operation executed."

    # Automated Audit
    Log-Info "Verifying checksums..."
    $SrcHash = Get-ChildItem $SrcDir -Recurse | Get-FileHash
    $DstHash = Get-ChildItem $DstDir -Recurse | Get-FileHash

    if (Compare-Object -ReferenceObject $SrcHash -DifferenceObject $DstHash -Property Hash -SyncWindow 0) {
        throw "Data integrity mismatch detected."
    }
    Log-Success "Integrity Audit Passed: 100% Consistency."

    Request-Observation "Sync Logic Check" "Verify '$DstDir': 'shard_1.dat' should be GONE. 'shard_new.dat' should be PRESENT."

} catch {
    Write-Header "CRITICAL FAILURE"
    Log-Error $_
    exit 1
} finally {
    # --- Teardown ---
    Write-Header "Phase 6: Decommissioning"
    Log-Info "Cleaning up validation workspace..."
    if (Test-Path $WorkDir) {
        Remove-Item $WorkDir -Recurse -Force
        Log-Success "Workspace removed."
    }
    Write-Host "`nOrbit Validation Protocol Finished." -ForegroundColor Magenta
}
