param(
    [int]$Port = 4444,
    [switch]$IncludeApply,
    [switch]$SkipBuild
)

$simsortDir = "C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort"
$sessionFile = Join-Path $PSScriptRoot "..\..\output\desktop\tauri-driver-session.json"

try {
    if (-not $SkipBuild) {
        Write-Output "TAURI_SMOKE_BUILD start=1"
        Push-Location $simsortDir
        & npm run tauri:build -- --debug
        $buildResult = $LASTEXITCODE
        Pop-Location
        if (-not $buildResult) {
            exit 1
        }
    }

    & (Join-Path $PSScriptRoot "run-tauri-webdriver.ps1") `
        -UseSmokeFixtures `
        -CleanSmokeProcesses `
        -Port $Port `
        -SessionFile $sessionFile

    if (-not $?) {
        exit 1
    }

    $env:SIMSUITE_WEBDRIVER_URL = "http://127.0.0.1:$Port"
    $env:SIMSUITE_TAURI_DRIVER_SESSION_FILE = $sessionFile

    Push-Location $simsortDir
    if ($IncludeApply) {
        $env:SIMSUITE_ALLOW_APPLY_SMOKE = "1"
        Write-Output "TAURI_SMOKE_START mode=apply url=$($env:SIMSUITE_WEBDRIVER_URL)"
        & node (Join-Path $PSScriptRoot "desktop-smoke.mjs") --include-apply
    } else {
        Write-Output "TAURI_SMOKE_START mode=base url=$($env:SIMSUITE_WEBDRIVER_URL)"
        & node (Join-Path $PSScriptRoot "desktop-smoke.mjs")
    }
    $smokeResult = $LASTEXITCODE
    Pop-Location

    Write-Output "TAURI_SMOKE_DONE exit=$smokeResult"
    exit $smokeResult

} finally {
    foreach ($name in @("tauri-driver", "msedgedriver", "simsuite", "SimSuite")) {
        try {
            Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force
        } catch {
        }
    }
}
