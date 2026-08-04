param(
    [int]$Port = 4444,
    [switch]$SkipBuild,
    [string]$OutputDir
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$sessionFile = Join-Path $repoRoot 'output\desktop\tauri-driver-session.json'
$outputRoot = if ($OutputDir) { $OutputDir } else { Join-Path $repoRoot 'output\desktop\library-proof' }

try {
    if (-not $SkipBuild) {
        Write-Output "DESKTOP_LIBRARY_PROOF_BUILD start=1"
        Push-Location $repoRoot
        try {
            & pnpm run tauri:build
            $buildResult = $LASTEXITCODE
        } finally {
            Pop-Location
        }
        if ($buildResult -ne 0) {
            exit $buildResult
        }
    }

    & (Join-Path $PSScriptRoot 'run-tauri-webdriver.ps1') `
        -UseSmokeFixtures `
        -CleanSmokeProcesses `
        -Port $Port `
        -SessionFile $sessionFile

    if (-not $?) {
        exit 1
    }

    $env:SIMSUITE_WEBDRIVER_URL = "http://127.0.0.1:$Port"
    $env:SIMSUITE_TAURI_DRIVER_SESSION_FILE = $sessionFile
    $env:SIMSUITE_DESKTOP_PROOF_OUTPUT = $outputRoot

    Push-Location $repoRoot
    Write-Output "DESKTOP_LIBRARY_PROOF_START url=$($env:SIMSUITE_WEBDRIVER_URL) output=$outputRoot"
    & node (Join-Path $PSScriptRoot 'desktop-library-proof.mjs')
    $proofResult = $LASTEXITCODE
    Pop-Location

    Write-Output "DESKTOP_LIBRARY_PROOF_DONE exit=$proofResult output=$outputRoot"
    exit $proofResult
} finally {
    foreach ($name in @('tauri-driver', 'msedgedriver', 'simsuite', 'SimSuite')) {
        try {
            Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force
        } catch {
        }
    }
}
