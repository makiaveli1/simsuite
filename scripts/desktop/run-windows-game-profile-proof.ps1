param(
    [string]$OutputDir
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'The native Windows game-profile proof must run in Windows PowerShell.'
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$outputRoot = if ($OutputDir) {
    [System.IO.Path]::GetFullPath($OutputDir)
} else {
    Join-Path $repoRoot 'output\desktop\windows-game-profile-proof'
}
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$runDir = Join-Path $outputRoot $timestamp
$logPath = Join-Path $runDir 'cargo-test.log'
$summaryPath = Join-Path $runDir 'summary.json'
$latestSummaryPath = Join-Path $outputRoot 'latest-summary.json'

New-Item -ItemType Directory -Force -Path $runDir | Out-Null

$cargoExecutable = (Get-Command cargo -CommandType Application -ErrorAction Stop).Source
$previousErrorActionPreference = $ErrorActionPreference
Push-Location $repoRoot
try {
    Write-Output "WINDOWS_GAME_PROFILE_PROOF_START output=$runDir"
    try {
        # Windows PowerShell 5.1 surfaces normal native stderr as NativeCommandError
        # when ErrorActionPreference is Stop. Capture both streams, then trust the
        # native exit code rather than treating Cargo progress output as failure.
        $ErrorActionPreference = 'Continue'
        $testOutput = @(
            & $cargoExecutable test `
                --manifest-path 'src-tauri/Cargo.toml' `
                'core::game_installation_candidate_detection::tests::' `
                -- `
                --include-ignored `
                --nocapture 2>&1
        )
        $testExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
} finally {
    Pop-Location
}

$testLines = @($testOutput | ForEach-Object { "$_" })
$testLines | Tee-Object -FilePath $logPath | Write-Output
if ($testExitCode -ne 0) {
    throw "Native Windows game-profile proof tests failed with exit code $testExitCode."
}

$markerPrefix = 'SIMSUITE_WINDOWS_GAME_PROFILE_PROOF_JSON='
$markerLine = $testLines |
    Where-Object { $_.Contains($markerPrefix) } |
    Select-Object -Last 1
if (-not $markerLine) {
    throw 'The native detector receipt marker was not found in the Rust test output.'
}
$markerIndex = $markerLine.IndexOf($markerPrefix)
$detectorJson = $markerLine.Substring($markerIndex + $markerPrefix.Length)
$detectorReceipt = $detectorJson | ConvertFrom-Json

$gitHead = ((& git -C $repoRoot rev-parse HEAD) | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or -not $gitHead) {
    throw 'Could not record the current Git revision.'
}

$summary = [ordered]@{
    schemaVersion = '1.0'
    proof = 'native_windows_game_profile_candidates'
    recordedAtUtc = (Get-Date).ToUniversalTime().ToString('o')
    gitHead = $gitHead
    testFilter = 'core::game_installation_candidate_detection::tests::'
    includedIgnoredTests = $true
    host = [ordered]@{
        osVersion = [System.Environment]::OSVersion.VersionString
        architecture = $env:PROCESSOR_ARCHITECTURE
        powershellVersion = $PSVersionTable.PSVersion.ToString()
    }
    detector = $detectorReceipt
    artifacts = [ordered]@{
        log = $logPath
        summary = $summaryPath
    }
}

$summaryJson = $summary | ConvertTo-Json -Depth 32
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($summaryPath, $summaryJson, $utf8NoBom)
[System.IO.File]::WriteAllText($latestSummaryPath, $summaryJson, $utf8NoBom)

Write-Output "WINDOWS_GAME_PROFILE_PROOF_DONE exit=0 summary=$summaryPath latest=$latestSummaryPath"
