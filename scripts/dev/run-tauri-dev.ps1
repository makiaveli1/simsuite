param(
    [int]$Port = 1420
)

$ErrorActionPreference = 'Stop'

$cleanupScript = Join-Path $PSScriptRoot 'cleanup-dev-port.ps1'
$tauriCommand = Join-Path $PSScriptRoot '..\..\node_modules\.bin\tauri.cmd'

if (-not (Test-Path $tauriCommand)) {
    throw "SimSuite could not find the local Tauri CLI at $tauriCommand. Run npm install first."
}

& $cleanupScript -Port $Port -Quiet

try {
    & $tauriCommand dev
    exit $LASTEXITCODE
} finally {
    try {
        & $cleanupScript -Port $Port -Quiet
    } catch {
        Write-Warning $_
    }
}
