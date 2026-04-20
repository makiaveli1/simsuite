$ErrorActionPreference = 'SilentlyContinue'
$proc = Get-Process -Id 112456
if (-not $proc) { Write-Host "Process not found"; exit 1 }
Write-Host "Process: SimSuite PID 112456"

# Check listening ports
$listening = Get-NetTCPConnection -State Listen | Where-Object { $_.OwningProcess -eq 112456 }
if ($listening) {
    Write-Host "Listening ports:"
    $listening | ForEach-Object { Write-Host "  Port: $($_.LocalPort)" }
} else {
    Write-Host "No listening ports found (app may still be starting)"
}
