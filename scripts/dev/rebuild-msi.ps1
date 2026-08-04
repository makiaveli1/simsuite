param(
    [string]$LogPath
)

# Windows-only helper for producing the native installer from any SimSuite clone.
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') {
    throw 'SimSuite MSI rebuilds must run from Windows PowerShell.'
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$outputRoot = Join-Path $repoRoot 'output\build'
if (-not $LogPath) {
    $LogPath = Join-Path $outputRoot 'simsuite-rebuild.log'
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogPath) | Out-Null

$start = Get-Date
Write-Host "Starting SimSuite MSI rebuild at $start"
Write-Host "Repository: $repoRoot"
Write-Host "Log: $LogPath"

Push-Location $repoRoot
try {
    & pnpm run tauri:build 2>&1 | Tee-Object -FilePath $LogPath
    $buildResult = $LASTEXITCODE
} finally {
    Pop-Location
}

$end = Get-Date
Write-Host "Build finished at $end"
Write-Host "Duration: $(($end - $start).TotalMinutes) minutes"

if ($buildResult -ne 0) {
    exit $buildResult
}
