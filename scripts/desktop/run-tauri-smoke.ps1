param(
    [int]$Port = 4444,
    [switch]$IncludeApply,
    [switch]$SkipBuild
)

$sessionFile = Join-Path $PSScriptRoot '..\..\output\desktop\tauri-driver-session.json'

try {
    if (-not $SkipBuild) {
        Write-Output "TAURI_SMOKE_BUILD start=1"
        & npm run tauri:build -- --debug
        if (-not $?) {
            exit 1
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

    if ($IncludeApply) {
        $env:SIMSUITE_ALLOW_APPLY_SMOKE = '1'
        Write-Output "TAURI_SMOKE_START mode=apply url=$($env:SIMSUITE_WEBDRIVER_URL)"
        & node (Join-Path $PSScriptRoot 'desktop-smoke.mjs') --include-apply
    } else {
        Write-Output "TAURI_SMOKE_START mode=base url=$($env:SIMSUITE_WEBDRIVER_URL)"
        & node (Join-Path $PSScriptRoot 'desktop-smoke.mjs')
    }

    Write-Output "TAURI_SMOKE_DONE exit=$LASTEXITCODE"

    exit $LASTEXITCODE
} finally {
    foreach ($name in @('tauri-driver', 'msedgedriver', 'simsuite', 'SimSuite')) {
        try {
            Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force
        } catch {
        }
    }
}
