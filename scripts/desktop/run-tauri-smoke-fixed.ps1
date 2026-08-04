param(
    [int]$Port = 4444,
    [switch]$IncludeApply,
    [switch]$SkipBuild
)

# Deprecated compatibility entry point. Keep old automation working while
# routing every run through the canonical, repo-relative smoke wrapper.
Write-Warning 'run-tauri-smoke-fixed.ps1 is deprecated; delegating to run-tauri-smoke.ps1.'
& (Join-Path $PSScriptRoot 'run-tauri-smoke.ps1') @PSBoundParameters
