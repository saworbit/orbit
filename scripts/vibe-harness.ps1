#requires -Version 5.1
<#
.SYNOPSIS
    End-to-end vibe test harness for Orbit's single-file copy.

.DESCRIPTION
    Exercises the behavioural claims from README.md and the single-file-copy
    design doc against a built orbit binary. Each check is a self-contained
    "harness" (H1..H13) that prepares fixtures in a fresh temp directory,
    runs orbit, and asserts an observable property.

    The harness is intentionally black-box: it only drives the binary and
    inspects filesystem + stdout/stderr/exit-code, so it cross-checks the
    Rust integration tests from outside the test process.

    Claims probed:
      - plan output is deterministic (incl. operation_id)
      - identical-destination copy is a true no-op (hash + mtime preserved)
      - N concurrent copies to the same new dest publish atomically
      - staging artifacts are cleaned up on success and on conflict
      - a mid-copy source modification is caught (source_changed)
      - large-file round-trip hash matches
      - --json output is well-formed NDJSON with schema_version/event/data
      - --verify size still refuses same-size different-content dest
      - operation_id varies with inputs
      - dest path is never visible with partial content during publish
      - staging file lives beside the destination
      - exit codes 0/2/3 match the documented mapping

.PARAMETER OrbitPath
    Path to the orbit executable. Defaults to <repo>/target/release/orbit.exe
    relative to this script's parent directory.

.EXAMPLE
    pwsh -NoProfile -File scripts/vibe-harness.ps1
    pwsh -NoProfile -File scripts/vibe-harness.ps1 -OrbitPath C:\build\orbit.exe
#>
[CmdletBinding()]
param(
    [string]$OrbitPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $OrbitPath) {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $onWindows = ($env:OS -eq 'Windows_NT') -or ((Get-Variable -Name IsWindows -Scope Global -ErrorAction SilentlyContinue) -and $IsWindows)
    $exeName = if ($onWindows) { 'orbit.exe' } else { 'orbit' }
    $OrbitPath = Join-Path $repoRoot ("target/release/" + $exeName)
}
$orbit = $OrbitPath
if (-not (Test-Path $orbit)) {
    throw "orbit binary missing: $orbit (build it with 'cargo build --release' or pass -OrbitPath)"
}

$root = Join-Path $env:TEMP ("orbit-harness-" + [guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $root | Out-Null
Write-Host "Harness root: $root"

$results = [System.Collections.Generic.List[object]]::new()
function Record([string]$name, [bool]$pass, [string]$detail = '') {
    $tag = if ($pass) { 'PASS' } else { 'FAIL' }
    Write-Host ("[{0}] {1}" -f $tag, $name)
    if ($detail) { Write-Host ("       {0}" -f ($detail -replace "`r?`n", "`n       ")) }
    $results.Add([pscustomobject]@{ Name = $name; Pass = $pass; Detail = $detail }) | Out-Null
}

function Run-Orbit {
    param([string[]]$CmdArgs)
    $out = New-TemporaryFile
    $err = New-TemporaryFile
    try {
        $proc = Start-Process -FilePath $orbit -ArgumentList $CmdArgs -NoNewWindow -Wait -PassThru -RedirectStandardOutput $out.FullName -RedirectStandardError $err.FullName
        return [pscustomobject]@{
            ExitCode = $proc.ExitCode
            Stdout = (Get-Content $out.FullName -Raw -ErrorAction SilentlyContinue)
            Stderr = (Get-Content $err.FullName -Raw -ErrorAction SilentlyContinue)
        }
    } finally {
        Remove-Item $out.FullName, $err.FullName -ErrorAction SilentlyContinue
    }
}

function New-FixtureBytes {
    param([string]$Path, [int]$SizeBytes, [int]$Seed = 1)
    $rng = [System.Random]::new($Seed)
    $buf = New-Object byte[] $SizeBytes
    $rng.NextBytes($buf)
    [System.IO.File]::WriteAllBytes($Path, $buf)
}

function Get-FileSha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLower()
}

# ============ H1: plan output determinism ============
# Claim: plan is deterministic. Running it N times with identical inputs
# should produce byte-identical stdout and identical operation_id in JSON.
function H1-PlanDeterminism {
    $dir = Join-Path $root 'h1'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes 65536 -Seed 42
    $dst = "$dir\dst.bin"

    $humanRuns = @()
    $jsonRuns = @()
    1..10 | ForEach-Object {
        $humanRuns += (Run-Orbit @('plan', "$dir\src.bin", $dst)).Stdout
        $jsonRuns += (Run-Orbit @('plan', "$dir\src.bin", $dst, '--json')).Stdout
    }

    $humanUnique = @($humanRuns | Select-Object -Unique)
    $jsonUnique = @($jsonRuns | Select-Object -Unique)
    $humanOk = $humanUnique.Count -eq 1
    $jsonOk = $jsonUnique.Count -eq 1

    # Operation_id stable
    $opIds = @()
    foreach ($line in $jsonRuns) {
        foreach ($ev in ($line -split "`n")) {
            if ($ev -match '"event":"plan_result"' -and $ev -match '"operation_id":"([a-f0-9]+)"') {
                $opIds += $matches[1]
            }
        }
    }
    $opUnique = @($opIds | Select-Object -Unique)
    $opOk = $opUnique.Count -eq 1 -and $opIds.Count -eq 10

    $detail = "human unique={0}, json unique={1}, op_ids unique={2} (sample: {3})" -f `
        $humanUnique.Count, $jsonUnique.Count, $opUnique.Count, (@($opIds)[0])
    Record 'H1 plan output is deterministic' ($humanOk -and $jsonOk -and $opOk) $detail
}

# ============ H2: no-op idempotence doesn't mutate dest ============
# Claim: identical existing dest is a successful no-op.
# Must leave content AND mtime unchanged.
function H2-NoopIdempotence {
    $dir = Join-Path $root 'h2'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes 32768 -Seed 7
    $dst = "$dir\dst.bin"
    $r1 = Run-Orbit @('copy', "$dir\src.bin", $dst)
    if ($r1.ExitCode -ne 0) {
        Record 'H2 no-op does not mutate dest' $false "initial copy failed exit=$($r1.ExitCode) stderr=$($r1.Stderr)"
        return
    }
    $hashBefore = Get-FileSha256 $dst
    $mtimeBefore = (Get-Item $dst).LastWriteTimeUtc.Ticks
    Start-Sleep -Milliseconds 1500   # ensure a touch would show up at FS mtime resolution

    $r2 = Run-Orbit @('copy', "$dir\src.bin", $dst)
    $hashAfter = Get-FileSha256 $dst
    $mtimeAfter = (Get-Item $dst).LastWriteTimeUtc.Ticks

    $claimsNoop = $r2.Stdout -match 'Already identical'
    $hashOk = $hashBefore -eq $hashAfter
    $mtimeOk = $mtimeBefore -eq $mtimeAfter

    $detail = "noop_message={0}, hash_preserved={1}, mtime_preserved={2}, mtime_delta_ticks={3}, exit={4}" -f `
        $claimsNoop, $hashOk, $mtimeOk, ($mtimeAfter - $mtimeBefore), $r2.ExitCode
    Record 'H2 no-op does not mutate dest' ($claimsNoop -and $hashOk -and $mtimeOk -and $r2.ExitCode -eq 0) $detail
}

# ============ H3: concurrent copies to same destination ============
# Claim: destination is published atomically without overwriting an existing path.
# N parallel copies to same new dest: exactly one should succeed as "copied";
# the rest should either succeed as "already identical" (publication after another won)
# or fail cleanly with destination_exists. No corrupted dest.
function H3-ConcurrentCopy {
    $dir = Join-Path $root 'h3'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes (4 * 1024 * 1024) -Seed 13
    $srcHash = Get-FileSha256 "$dir\src.bin"
    $dst = "$dir\dst.bin"

    $jobs = 1..8 | ForEach-Object {
        Start-Job -ScriptBlock {
            param($orbit, $src, $dst)
            $o = & $orbit copy $src $dst --json 2>&1
            [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = ($o -join "`n") }
        } -ArgumentList $orbit, "$dir\src.bin", $dst
    }
    $outputs = $jobs | Wait-Job | Receive-Job
    $jobs | Remove-Job

    $copiedArr = @($outputs | Where-Object { $_.Output -match '"status":"copied"' })
    $identicalArr = @($outputs | Where-Object { $_.Output -match '"status":"already_identical"' })
    $failures = @($outputs | Where-Object { $_.ExitCode -ne 0 })
    $copied = $copiedArr.Count
    $identical = $identicalArr.Count
    $failureCodes = ($failures | ForEach-Object {
        if ($_.Output -match '"code":"([a-z_]+)"') { $matches[1] } else { 'unknown' }
    }) -join ','

    $dstHash = Get-FileSha256 $dst
    $contentOk = $dstHash -eq $srcHash
    $exactlyOneCopied = $copied -eq 1
    $unclean = @($failures | Where-Object { $_.Output -notmatch '"code":"(destination_exists|verification_failed|source_changed|staging_changed)"' -and $_.Output -notmatch '"status":"already_identical"' })
    $cleanFailures = $unclean.Count -eq 0

    $detail = "copied={0} identical={1} failures={2} failure_codes={3} dst_hash_matches={4}" -f `
        $copied, $identical, $failures.Count, $failureCodes, $contentOk
    Record 'H3 concurrent copy publishes atomically, dest consistent' ($exactlyOneCopied -and $contentOk -and $cleanFailures) $detail
}

# ============ H4: staging artifacts cleanup ============
# Claim: staging files are cleaned up on success. (Reasonable extension of claim.)
function H4-StagingCleanupOnSuccess {
    $dir = Join-Path $root 'h4'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes 16384 -Seed 99
    $r = Run-Orbit @('copy', "$dir\src.bin", "$dir\dst.bin")
    $leftovers = @(Get-ChildItem $dir -Force | Where-Object { $_.Name -ne 'src.bin' -and $_.Name -ne 'dst.bin' })
    $leftoverNames = ($leftovers | ForEach-Object { $_.Name }) -join ', '
    $detail = "exit={0}, extra_files_in_dest_dir=[{1}]" -f $r.ExitCode, $leftoverNames
    Record 'H4 staging artifacts cleaned up after success' ($r.ExitCode -eq 0 -and $leftovers.Count -eq 0) $detail
}

# ============ H5: source-change race ============
# Claim (commit 2abf672): copy is hardened against file identity races.
# Race: start a copy of a BIG file; mid-flight, truncate/overwrite source.
# Orbit must either succeed (and dest hash == original source hash)
# or refuse with source_changed / staging_changed / verification_failed.
# Must NOT silently produce a dest that doesn't match either source version.
function H5-SourceChangeRace {
    $dir = Join-Path $root 'h5'
    New-Item -ItemType Directory -Path $dir | Out-Null
    # 1GB source so orbit spends real seconds hashing+copying.
    $srcSize = 1024 * 1024 * 1024
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes $srcSize -Seed 55
    Copy-Item "$dir\src.bin" "$dir\src_snapshot.bin"
    $dst = "$dir\dst.bin"

    # Start the stomper first, running in a tight loop. It opens-writes-closes
    # repeatedly so SOME iteration overlaps with orbit's hash/copy phase.
    # It stomps the LAST 1MB (far enough into the file that orbit's hashing
    # will cross it mid-pass).
    $stomperScript = {
        param($srcPath, $offsetFromEnd, $stopFlag)
        $size = (Get-Item $srcPath).Length
        $offset = $size - $offsetFromEnd
        $iters = 0
        while (-not (Test-Path $stopFlag)) {
            $iters++
            try {
                $fs = [System.IO.File]::Open($srcPath, 'Open', 'Write', 'ReadWrite')
                try {
                    $fs.Seek($offset, 'Begin') | Out-Null
                    $byte = [byte](($iters % 253) + 1)   # avoid 0, 0xFF to keep bytes "stomp-looking"
                    $buf = New-Object byte[] (64 * 1024)
                    for ($i=0; $i -lt $buf.Length; $i++) { $buf[$i] = $byte }
                    $fs.Write($buf, 0, $buf.Length)
                    $fs.Flush($true)
                } finally { $fs.Dispose() }
            } catch { Start-Sleep -Milliseconds 1 }
        }
        $iters
    }
    $stopFlag = "$dir\stop.flag"
    $stomperJob = Start-Job -ScriptBlock $stomperScript -ArgumentList "$dir\src.bin", (1 * 1024 * 1024), $stopFlag

    # Give the stomper a head start so it's already pounding on the file.
    Start-Sleep -Milliseconds 200

    $orbitJob = Start-Job -ScriptBlock {
        param($orbitPath, $src, $dst)
        $out = & $orbitPath copy $src $dst --json 2>&1
        [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = ($out -join "`n") }
    } -ArgumentList $orbit, "$dir\src.bin", $dst

    $out = $orbitJob | Wait-Job | Receive-Job
    $orbitJob | Remove-Job
    # Stop stomper
    New-Item -ItemType File -Path $stopFlag | Out-Null
    $stomperIters = $stomperJob | Wait-Job | Receive-Job
    $stomperJob | Remove-Job
    Remove-Item $stopFlag -ErrorAction SilentlyContinue

    $exit = $out.ExitCode
    $output = $out.Output

    $snapshotHash = Get-FileSha256 "$dir\src_snapshot.bin"
    $finalSrcHash = Get-FileSha256 "$dir\src.bin"
    $dstExists = Test-Path $dst
    $dstHash = if ($dstExists) { Get-FileSha256 $dst } else { '' }

    $errCode = ''
    if ($output -match '"code":"([a-z_]+)"') { $errCode = $matches[1] }

    # The ONLY safe outcomes:
    #   a) exit=0 and dst hash matches what the source WAS at some consistent instant.
    #      For a race against external modification, orbit's hash was computed from
    #      the source at some point. The STAGED file was then verified against that hash.
    #      So orbit's own claim is: dst's content hash matches whatever orbit computed.
    #      This is only verifiable from outside if we hash the dst and compare the orbit
    #      claim. Here we require: the dst hash matches EITHER the snapshot or any version
    #      of the stomped source. If it matches neither, orbit shipped something internally
    #      inconsistent (dst != hash it verified).
    #   b) exit=3 and NO dst was published (clean refusal: source_changed/staging_changed/verification_failed)
    # UNSAFE outcomes:
    #   - exit=0 but dst does not match the hash orbit claimed (we can't check that here directly,
    #     but we DO know: the dst must be internally consistent — content hash = what orbit
    #     hashed. Since mid-copy writes while hashing produce an unspecified hash, the harness
    #     mainly asks: did orbit publish an inconsistent dst?)
    #
    # To tell internal consistency, re-run orbit's own hash on the dst and demand that it
    # equals the digest orbit reported in --json. We extract that digest from the plan JSON
    # (plan embeds "source_digest" per entry — but the copy command only prints copy_result,
    # not plan_entry. So we lean on the simpler check: dst must match either the orig snapshot
    # OR the final stomped source. If neither, orbit shipped something it never saw whole.)

    $ok = $false
    $reason = ''
    if ($exit -eq 0) {
        if ($dstExists -and ($dstHash -eq $snapshotHash -or $dstHash -eq $finalSrcHash)) {
            $ok = $true
            $reason = 'succeeded; dst matches a known source snapshot'
        } elseif ($dstExists) {
            $reason = "SUCCEEDED but dst matches neither original nor final source (orbit shipped bytes it never saw as a whole: dst=$dstHash)"
        }
    } elseif ($exit -eq 3) {
        if (-not $dstExists) { $ok = $true; $reason = "refused ($errCode); no dest" }
        else { $reason = "refused ($errCode) but dst was published anyway" }
    } else {
        $reason = "unexpected exit=$exit code=$errCode"
    }

    $detail = "stomper_iters={0}, exit={1}, err_code={2}, dst_exists={3}, dst==orig={4}, dst==final_src={5}, reason=$reason" -f `
        $stomperIters, $exit, $errCode, $dstExists, ($dstHash -eq $snapshotHash), ($dstHash -eq $finalSrcHash)
    Record 'H5 source-change race detected or benign' $ok $detail
}

# ============ H6: large file round-trip ============
# Just a sanity: for a ~300MB file the stream-copy preserves bytes.
function H6-LargeFileHash {
    $dir = Join-Path $root 'h6'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes (300 * 1024 * 1024) -Seed 77
    $srcHash = Get-FileSha256 "$dir\src.bin"
    $r = Run-Orbit @('copy', "$dir\src.bin", "$dir\dst.bin")
    $dstHash = Get-FileSha256 "$dir\dst.bin"
    Record 'H6 large file (300MB) round-trip hash match' ($r.ExitCode -eq 0 -and $srcHash -eq $dstHash) ("exit={0} src=$srcHash dst=$dstHash" -f $r.ExitCode)
}

# ============ H7: JSON NDJSON schema sanity ============
# Claim (per tests): JSON is newline-delimited JSON, one object per line, each with schema_version/event.
function H7-JsonNdjson {
    $dir = Join-Path $root 'h7'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes 1024 -Seed 1
    $okAll = $true
    $detail = @()
    foreach ($scenario in @(
        @{ Args = @('plan', "$dir\src.bin", "$dir\dst.bin", '--json'); Name = 'plan ok' },
        @{ Args = @('copy', "$dir\src.bin", "$dir\dst.bin", '--json'); Name = 'copy ok' },
        @{ Args = @('copy', "$dir\missing.bin", "$dir\o.bin", '--json'); Name = 'copy err' }
    )) {
        $r = Run-Orbit $scenario.Args
        $lines = ($r.Stdout -split "`r?`n") | Where-Object { $_ -ne '' }
        foreach ($line in $lines) {
            try {
                $obj = $line | ConvertFrom-Json -ErrorAction Stop
                if (-not ($obj.PSObject.Properties.Name -contains 'schema_version') -or `
                    -not ($obj.PSObject.Properties.Name -contains 'event') -or `
                    -not ($obj.PSObject.Properties.Name -contains 'data')) {
                    $okAll = $false
                    $detail += "$($scenario.Name): line missing required keys: $line"
                }
                if ($obj.schema_version -ne 1) {
                    $okAll = $false
                    $detail += "$($scenario.Name): schema_version=$($obj.schema_version)"
                }
            } catch {
                $okAll = $false
                $detail += "$($scenario.Name): invalid JSON line: $line"
            }
        }
    }
    Record 'H7 JSON output is well-formed NDJSON' $okAll ($detail -join '; ')
}

# ============ H8: verify-size still rejects same-size different-content ============
function H8-VerifySizeRejectsMismatch {
    $dir = Join-Path $root 'h8'
    New-Item -ItemType Directory -Path $dir | Out-Null
    [IO.File]::WriteAllBytes("$dir\src.bin", @(1..32 | ForEach-Object { [byte]$_ }))
    [IO.File]::WriteAllBytes("$dir\dst.bin", @(32..1 | ForEach-Object { [byte]$_ }))
    $srcHash = Get-FileSha256 "$dir\src.bin"
    $dstHashBefore = Get-FileSha256 "$dir\dst.bin"
    $r = Run-Orbit @('copy', "$dir\src.bin", "$dir\dst.bin", '--verify', 'size')
    $dstHashAfter = Get-FileSha256 "$dir\dst.bin"
    $unchanged = $dstHashBefore -eq $dstHashAfter
    Record 'H8 --verify size still refuses same-size different-content dest' ($r.ExitCode -eq 3 -and $unchanged) ("exit={0} dst_unchanged={1}" -f $r.ExitCode, $unchanged)
}

# ============ H9: operation_id depends on inputs (not just stable) ============
# A hollow determinism harness would also report OK if operation_id was constant.
# Verify operation_id changes when source or dest changes.
function H9-OperationIdSensitivity {
    $dir = Join-Path $root 'h9'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\a.bin" -SizeBytes 2048 -Seed 1
    New-FixtureBytes -Path "$dir\b.bin" -SizeBytes 2048 -Seed 2

    function Op($opArgs) {
        $r = Run-Orbit $opArgs
        foreach ($line in ($r.Stdout -split "`r?`n")) {
            if ($line -match '"event":"plan_result".*"operation_id":"([a-f0-9]+)"') { return $matches[1] }
        }
        return $null
    }

    $id_a_x = Op @('plan', "$dir\a.bin", "$dir\x.bin", '--json')
    $id_a_y = Op @('plan', "$dir\a.bin", "$dir\y.bin", '--json')
    $id_b_x = Op @('plan', "$dir\b.bin", "$dir\x.bin", '--json')

    $unique = @(@($id_a_x, $id_a_y, $id_b_x) | Select-Object -Unique)
    $allDistinct = $unique.Count -eq 3
    Record 'H9 operation_id varies with inputs' $allDistinct "a->x=$id_a_x a->y=$id_a_y b->x=$id_b_x"
}

# ============ H10: staging artifacts cleanup after mid-copy source lock ============
# If we hold an exclusive lock on the destination's directory contents during a copy,
# or if copy fails partway, no partial dest file should remain.
# We simulate a failure by racing: pre-create destination between resolve and publish is tough,
# but we can at least verify that a FAILED copy (dest already different) leaves no .staging artifact.
function H10-StagingCleanupOnConflict {
    $dir = Join-Path $root 'h10'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes 65536 -Seed 3
    New-FixtureBytes -Path "$dir\dst.bin" -SizeBytes 65536 -Seed 4
    $r = Run-Orbit @('copy', "$dir\src.bin", "$dir\dst.bin")
    $leftovers = @(Get-ChildItem $dir -Force | Where-Object { $_.Name -ne 'src.bin' -and $_.Name -ne 'dst.bin' })
    $names = ($leftovers | ForEach-Object { $_.Name }) -join ', '
    Record 'H10 conflict refusal leaves no staging artifacts' ($r.ExitCode -eq 3 -and $leftovers.Count -eq 0) ("exit=$($r.ExitCode) extras=[$names]")
}

# ============ H11: publish atomicity — dest path never has partial content ============
# Claim: dest path is only published after staging+verify. A watcher at the
# dest path should either see no file, or see a complete, verified file.
# We watch the dest path repeatedly during a large copy; any partial sighting
# (file exists but shorter than src.size) is a violation.
function H11-PublishAtomicity {
    $dir = Join-Path $root 'h11'
    New-Item -ItemType Directory -Path $dir | Out-Null
    $srcSize = 500 * 1024 * 1024
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes $srcSize -Seed 101
    $dst = "$dir\dst.bin"

    $job = Start-Job -ScriptBlock {
        param($orbitPath, $src, $dst)
        $out = & $orbitPath copy $src $dst 2>&1
        $LASTEXITCODE
    } -ArgumentList $orbit, "$dir\src.bin", $dst

    $partialSightings = 0
    $samples = 0
    while ($job.State -eq 'Running') {
        $samples++
        if (Test-Path $dst) {
            $size = (Get-Item $dst -ErrorAction SilentlyContinue).Length
            if ($size -ne $null -and $size -lt $srcSize) {
                $partialSightings++
            }
        }
        Start-Sleep -Milliseconds 10
    }
    $exit = $job | Wait-Job | Receive-Job
    $job | Remove-Job

    $finalExists = Test-Path $dst
    $finalSize = if ($finalExists) { (Get-Item $dst).Length } else { 0 }
    $detail = "samples={0}, partial_sightings={1}, final_exists={2}, final_size={3}, exit={4}" -f `
        $samples, $partialSightings, $finalExists, $finalSize, $exit
    Record 'H11 dest path never visible with partial content' ($partialSightings -eq 0 -and $exit -eq 0 -and $finalSize -eq $srcSize) $detail
}

# ============ H12: staging file lives in dest's parent directory ============
# Claim (from README): "writes a uniquely named staging file beside the destination".
# During a copy, we should see a file in the dest's parent dir whose name isn't
# the dest's basename, and whose size grows toward the source size.
function H12-StagingBesideDestination {
    $dir = Join-Path $root 'h12'
    New-Item -ItemType Directory -Path $dir | Out-Null
    $srcSize = 200 * 1024 * 1024
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes $srcSize -Seed 202
    $dst = "$dir\sub\dst.bin"
    New-Item -ItemType Directory -Path (Split-Path $dst) | Out-Null

    $job = Start-Job -ScriptBlock {
        param($orbitPath, $src, $dst)
        & $orbitPath copy $src $dst 2>&1 | Out-Null
        $LASTEXITCODE
    } -ArgumentList $orbit, "$dir\src.bin", $dst

    $stageSeenInParent = $false
    $stageSeenElsewhere = $false
    $stageNames = @{}
    while ($job.State -eq 'Running') {
        # Watch dest parent
        Get-ChildItem (Split-Path $dst) -Force -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -ne 'dst.bin' } | ForEach-Object {
                $stageSeenInParent = $true
                $stageNames[$_.Name] = $true
            }
        Start-Sleep -Milliseconds 10
    }
    $exit = $job | Wait-Job | Receive-Job
    $job | Remove-Job

    $afterStageFiles = @(Get-ChildItem (Split-Path $dst) -Force | Where-Object { $_.Name -ne 'dst.bin' })
    $cleanedUp = $afterStageFiles.Count -eq 0

    $detail = "stage_seen_beside_dst={0}, stage_names=[{1}], cleaned_up_after={2}, exit={3}" -f `
        $stageSeenInParent, ($stageNames.Keys -join ','), $cleanedUp, $exit
    Record 'H12 staging file lives in dest parent and is cleaned up' ($stageSeenInParent -and $cleanedUp -and $exit -eq 0) $detail
}

# ============ H13: documented exit-code mapping ============
# From tests: 0 success, 2 CLI arg errors, 3 domain errors.
function H13-ExitCodeMapping {
    $dir = Join-Path $root 'h13'
    New-Item -ItemType Directory -Path $dir | Out-Null
    New-FixtureBytes -Path "$dir\src.bin" -SizeBytes 1024 -Seed 1

    $cases = @(
        @{ Desc = 'success copy'; Args = @('copy', "$dir\src.bin", "$dir\new.bin"); Expect = 0 },
        @{ Desc = 'missing args'; Args = @('copy'); Expect = 2 },
        @{ Desc = 'unknown flag'; Args = @('copy', '--not-a-flag', "$dir\src.bin", "$dir\x.bin"); Expect = 2 },
        @{ Desc = 'bad verify value'; Args = @('copy', '--verify', 'bogus', "$dir\src.bin", "$dir\x.bin"); Expect = 2 },
        @{ Desc = 'source missing (domain)'; Args = @('copy', "$dir\nope.bin", "$dir\x.bin"); Expect = 3 },
        @{ Desc = 'parent missing (domain)'; Args = @('copy', "$dir\src.bin", "$dir\nodir\x.bin"); Expect = 3 }
    )
    $allOk = $true
    $msgs = @()
    foreach ($case in $cases) {
        $r = Run-Orbit $case.Args
        if ($r.ExitCode -ne $case.Expect) {
            $allOk = $false
            $msgs += ("{0}: got {1}, want {2}" -f $case.Desc, $r.ExitCode, $case.Expect)
        }
    }
    Record 'H13 exit codes match documented mapping (0/2/3)' $allOk ($msgs -join '; ')
}

# ============ Run all ============
H1-PlanDeterminism
H2-NoopIdempotence
H3-ConcurrentCopy
H4-StagingCleanupOnSuccess
H5-SourceChangeRace
H6-LargeFileHash
H7-JsonNdjson
H8-VerifySizeRejectsMismatch
H9-OperationIdSensitivity
H10-StagingCleanupOnConflict
H11-PublishAtomicity
H12-StagingBesideDestination
H13-ExitCodeMapping

Write-Host ""
Write-Host "===== Summary ====="
$passArr = @($results | Where-Object { $_.Pass })
$failArr = @($results | Where-Object { -not $_.Pass })
$pass = $passArr.Count
$fail = $failArr.Count
Write-Host "Passed: $pass"
Write-Host "Failed: $fail"
if ($fail -gt 0) {
    $failArr | ForEach-Object {
        Write-Host ("  - {0}: {1}" -f $_.Name, $_.Detail)
    }
}
Write-Host "Harness root (left in place): $root"
if ($fail -gt 0) { exit 1 } else { exit 0 }
