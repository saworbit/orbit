# ORBIT Sync Benchmark Script
# Benchmark sync performance with various configurations

param(
    [int]$SizeMB = 100,
    [string]$Mode = "sync",
    [string]$CheckMode = "modtime"
)

$ErrorActionPreference = "Stop"

Write-Host "ORBIT Sync Benchmark" -ForegroundColor Cyan
Write-Host "====================" -ForegroundColor Cyan
Write-Host ""

# Create temporary directories
$TempBase = [System.IO.Path]::GetTempPath()
$TestDir = Join-Path $TempBase "orbit_bench_$(Get-Random)"
$SrcDir = Join-Path $TestDir "src"
$DestDir = Join-Path $TestDir "dest"

Write-Host "Creating test directories in: $TestDir" -ForegroundColor Yellow
New-Item -ItemType Directory -Path $SrcDir -Force | Out-Null

# Generate test data
Write-Host "Generating ${SizeMB}MB of test data..." -ForegroundColor Yellow

# Create multiple files of varying sizes
$TotalBytes = $SizeMB * 1024 * 1024
$FileCount = 100
$BytesPerFile = [math]::Floor($TotalBytes / $FileCount)

for ($i = 1; $i -le $FileCount; $i++) {
    $FileName = "file_$($i.ToString('D4')).bin"
    $FilePath = Join-Path $SrcDir $FileName

    # Create file with random data
    $Buffer = New-Object byte[] $BytesPerFile
    $Random = [System.Random]::new()
    $Random.NextBytes($Buffer)
    [System.IO.File]::WriteAllBytes($FilePath, $Buffer)

    if ($i % 20 -eq 0) {
        Write-Host "  Created $i/$FileCount files..." -ForegroundColor Gray
    }
}

Write-Host "Test data created: $FileCount files, ${SizeMB}MB total" -ForegroundColor Green
Write-Host ""

# Build ORBIT in release mode
Write-Host "Building ORBIT (release mode)..." -ForegroundColor Yellow
cargo build --release 2>&1 | Out-Null

# Run benchmark
Write-Host ""
Write-Host "Running benchmark: --mode $Mode --check $CheckMode" -ForegroundColor Cyan
Write-Host ""

$StopWatch = [System.Diagnostics.Stopwatch]::StartNew()

& cargo run --release -- `
    --source $SrcDir `
    --dest $DestDir `
    --recursive `
    --mode $Mode `
    --check $CheckMode `
    --parallel 4

$StopWatch.Stop()

$ElapsedSec = $StopWatch.Elapsed.TotalSeconds
$SpeedMBps = [math]::Round($SizeMB / $ElapsedSec, 2)

Write-Host ""
Write-Host "Benchmark Results" -ForegroundColor Cyan
Write-Host "-----------------" -ForegroundColor Cyan
Write-Host "Data size:    ${SizeMB}MB"
Write-Host "File count:   $FileCount"
Write-Host "Mode:         $Mode"
Write-Host "Check:        $CheckMode"
Write-Host "Duration:     $([math]::Round($ElapsedSec, 2)) seconds"
Write-Host "Throughput:   ${SpeedMBps} MB/s" -ForegroundColor Green
Write-Host ""

# Run second sync to test skip behavior
Write-Host "Running second sync (should skip all)..." -ForegroundColor Yellow
$StopWatch.Restart()

& cargo run --release -- `
    --source $SrcDir `
    --dest $DestDir `
    --recursive `
    --mode $Mode `
    --check $CheckMode `
    --parallel 4

$StopWatch.Stop()

Write-Host "Second sync duration: $([math]::Round($StopWatch.Elapsed.TotalSeconds, 2)) seconds" -ForegroundColor Green
Write-Host ""

# Cleanup
Write-Host "Cleaning up test directories..." -ForegroundColor Yellow
Remove-Item -Recurse -Force $TestDir

Write-Host "Benchmark complete!" -ForegroundColor Cyan
