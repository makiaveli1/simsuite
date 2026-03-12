param(
    [int]$Port = 4444,
    [switch]$UseSmokeFixtures
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

function New-SmokeTs4script {
    param(
        [string]$Path,
        [string]$Version
    )

    $workingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
    $payloadRoot = Join-Path $workingRoot 'payload'
    $zipPath = Join-Path $workingRoot 'script.zip'
    New-Item -ItemType Directory -Force -Path $payloadRoot | Out-Null
    Set-Content -Encoding UTF8 -Path (Join-Path $payloadRoot 'version.txt') -Value "MC Command Center $Version"
    Compress-Archive -Path (Join-Path $payloadRoot '*') -DestinationPath $zipPath -Force
    Move-Item -Force $zipPath $Path
    Remove-Item -Recurse -Force $workingRoot
}

function Write-SmokePackage {
    param(
        [string]$Path,
        [string]$Content
    )

    Set-Content -Encoding UTF8 -Path $Path -Value $Content
}

function New-SmokeZip {
    param(
        [string]$SourceRoot,
        [string]$ZipPath
    )

    if (Test-Path $ZipPath) {
        Remove-Item -Force $ZipPath
    }
    Compress-Archive -Path (Join-Path $SourceRoot '*') -DestinationPath $ZipPath -Force
}

function Initialize-SmokeFixtures {
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ("simsuite-desktop-smoke-" + [Guid]::NewGuid().ToString('N'))
    $appData = Join-Path $root 'appdata'
    $downloads = Join-Path $root 'downloads'
    $mods = Join-Path $root 'Mods'
    $installedMccc = Join-Path $mods 'MCCC'
    $incomingRoot = Join-Path $root 'incoming-mccc'
    $blockedRoot = Join-Path $root 'blocked-mccc'

    foreach ($path in @($appData, $downloads, $installedMccc, $incomingRoot, $blockedRoot)) {
        New-Item -ItemType Directory -Force -Path $path | Out-Null
    }

    New-SmokeTs4script -Path (Join-Path $installedMccc 'mc_cmd_center.ts4script') -Version '2025.9.0'
    Write-SmokePackage -Path (Join-Path $installedMccc 'mc_cmd_center.package') -Content 'installed mccc package'
    Write-SmokePackage -Path (Join-Path $installedMccc 'mc_woohoo.package') -Content 'installed woohoo package'
    Write-SmokePackage -Path (Join-Path $installedMccc 'mc_settings.cfg') -Content 'setting=true'

    New-SmokeTs4script -Path (Join-Path $incomingRoot 'mc_cmd_center.ts4script') -Version '2026.1.1'
    Write-SmokePackage -Path (Join-Path $incomingRoot 'mc_cmd_center.package') -Content 'incoming mccc package'
    Write-SmokePackage -Path (Join-Path $incomingRoot 'mc_woohoo.package') -Content 'incoming woohoo package'
    New-SmokeZip -SourceRoot $incomingRoot -ZipPath (Join-Path $downloads 'MCCC_Update_Test_2026_1_1.zip')

    Write-SmokePackage -Path (Join-Path $blockedRoot 'mc_woohoo.package') -Content 'partial woohoo only'
    New-SmokeZip -SourceRoot $blockedRoot -ZipPath (Join-Path $downloads 'MCCC_Partial_Blocked_Test.zip')

    return @{
        Root = $root
        AppData = $appData
        Downloads = $downloads
        Mods = $mods
    }
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

$fixture = $null
if ($UseSmokeFixtures -or $env:SIMSUITE_USE_SMOKE_FIXTURES -eq '1') {
    $fixture = Initialize-SmokeFixtures
    $env:SIMSUITE_APP_DATA_DIR = $fixture.AppData
    $env:SIMSUITE_DOWNLOADS_PATH = $fixture.Downloads
    $env:SIMSUITE_MODS_PATH = $fixture.Mods
}

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

if ($fixture) {
    Write-Output "TAURI_DRIVER_FIXTURES root=$($fixture.Root) downloads=$($fixture.Downloads) mods=$($fixture.Mods)"
}
Write-Output "TAURI_DRIVER_READY pid=$($process.Id) url=$statusUrl"
