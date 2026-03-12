param(
    [int]$Port = 4444
)

function Resolve-ToolPath {
    param(
        [string]$EnvName,
        [string[]]$Candidates
    )

    $envValue = [Environment]::GetEnvironmentVariable($EnvName)
    if ($envValue -and (Test-Path $envValue)) {
        return $envValue
    }

    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }

    return $null
}

$userHome = $env:USERPROFILE
$tauriDriverPath = Resolve-ToolPath `
    -EnvName 'SIMSUITE_TAURI_DRIVER_PATH' `
    -Candidates @(
        (Join-Path $userHome '.cargo\bin\tauri-driver.exe'),
        (Join-Path $userHome '.cargo\bin\tauri-driver')
    )
$edgeDriverPath = Resolve-ToolPath `
    -EnvName 'SIMSUITE_MSEDGEDRIVER_PATH' `
    -Candidates @(
        (Join-Path $userHome '.codex\tools\msedgedriver\msedgedriver.exe'),
        (Join-Path $userHome '.codex\tools\edgedriver\msedgedriver.exe')
    )

if (-not $tauriDriverPath) {
    Write-Error 'SimSuite could not find tauri-driver. Set SIMSUITE_TAURI_DRIVER_PATH or install tauri-driver into %USERPROFILE%\.cargo\bin.'
    exit 1
}

if (-not $edgeDriverPath) {
    Write-Error 'SimSuite could not find msedgedriver.exe. Set SIMSUITE_MSEDGEDRIVER_PATH to the matching Edge driver.'
    exit 1
}

$arguments = @('--native-driver', $edgeDriverPath)
if ($Port -ne 4444) {
    $arguments += @('--port', $Port)
}

$process = Start-Process `
    -FilePath $tauriDriverPath `
    -ArgumentList $arguments `
    -PassThru `
    -WindowStyle Hidden

$statusUrl = "http://127.0.0.1:$Port/status"
$ready = $false
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    Start-Sleep -Milliseconds 500
    if ($process.HasExited) {
        Write-Error "tauri-driver exited early with code $($process.ExitCode)."
        exit 1
    }

    try {
        Invoke-WebRequest -UseBasicParsing -Uri $statusUrl | Out-Null
        $ready = $true
        break
    } catch {
    }
}

if (-not $ready) {
    try {
        Stop-Process -Id $process.Id -Force
    } catch {
    }
    Write-Error "tauri-driver did not become ready at $statusUrl."
    exit 1
}

Write-Output "TAURI_DRIVER_READY pid=$($process.Id) url=$statusUrl"
