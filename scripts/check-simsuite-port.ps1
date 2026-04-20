$ErrorActionPreference = 'SilentlyContinue'
$port = Get-NetTCPConnection -State Listen | Where-Object { $_.OwningProcess -eq 104668 } | Select-Object -ExpandProperty LocalPort
if ($port) {
    Write-Host "SimSuite listening on port: $port"
} else {
    Write-Host "No listening ports found for SimSuite (PID 104668)"
}
