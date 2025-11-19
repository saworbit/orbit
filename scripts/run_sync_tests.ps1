# ORBIT Sync/Mirror Test Script for Windows
# Run comprehensive tests for sync and mirror features

param(
    [switch]$Verbose,
    [switch]$Release,
    [string]$Filter = ""
)

$ErrorActionPreference = "Stop"

Write-Host "ORBIT Sync/Mirror Test Suite" -ForegroundColor Cyan
Write-Host "=============================" -ForegroundColor Cyan
Write-Host ""

# Build flags
$BuildFlags = @()
if ($Release) {
    $BuildFlags += "--release"
    Write-Host "Building in RELEASE mode" -ForegroundColor Yellow
} else {
    Write-Host "Building in DEBUG mode" -ForegroundColor Yellow
}

# Run unit tests for resilient_sync module
Write-Host ""
Write-Host "Running resilient_sync unit tests..." -ForegroundColor Green
$TestArgs = @("test", "resilient_sync", "--lib")
if ($Verbose) { $TestArgs += "--", "--nocapture" }
& cargo $TestArgs

# Run filter tests
Write-Host ""
Write-Host "Running filter tests..." -ForegroundColor Green
$TestArgs = @("test", "filter", "--lib")
if ($Verbose) { $TestArgs += "--", "--nocapture" }
& cargo $TestArgs

# Run delta tests
Write-Host ""
Write-Host "Running delta detection tests..." -ForegroundColor Green
$TestArgs = @("test", "delta", "--lib")
if ($Verbose) { $TestArgs += "--", "--nocapture" }
& cargo $TestArgs

# Run integration tests
Write-Host ""
Write-Host "Running sync/mirror integration tests..." -ForegroundColor Green
$TestArgs = @("test", "--test", "sync_mirror_tests")
if ($Verbose) { $TestArgs += "--", "--nocapture" }
if ($Filter) { $TestArgs += "--", $Filter }
& cargo $TestArgs

Write-Host ""
Write-Host "All sync/mirror tests completed!" -ForegroundColor Cyan
