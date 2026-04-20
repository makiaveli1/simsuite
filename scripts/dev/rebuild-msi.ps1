# SimSuite MSI rebuild script
$ErrorActionPreference = 'Continue'
$start = Get-Date
Write-Host "Starting MSI rebuild at $start"

cd C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort

$env:Path = "C:\Windows\System32;C:\Windows\System32\WindowsPowerShell\v1.0;C:\Program Files\nodejs;$env:APPDATA\npm;$env:LOCALAPPDATA\Programs\pnpm;$env:Path"

npm run tauri:build 2>&1 | Out-File -FilePath C:\Users\likwi\simsuite-rebuild.log -Encoding utf8

$end = Get-Date
Write-Host "Build finished at $end"
Write-Host "Duration: $(($end - $start).TotalMinutes) minutes"
