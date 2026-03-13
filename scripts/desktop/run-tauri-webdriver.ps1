param(
    [int]$Port = 4444,
    [switch]$UseSmokeFixtures,
    [switch]$CleanSmokeProcesses,
    [string]$SessionFile = (Join-Path $PSScriptRoot '..\..\output\desktop\tauri-driver-session.json')
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
        [string]$Version,
        [string]$Marker = 'MC Command Center'
    )

    $workingRoot = Join-Path ([System.IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
    $payloadRoot = Join-Path $workingRoot 'payload'
    $zipPath = Join-Path $workingRoot 'script.zip'
    New-Item -ItemType Directory -Force -Path $payloadRoot | Out-Null
    Set-Content -Encoding UTF8 -Path (Join-Path $payloadRoot 'version.txt') -Value "$Marker $Version"
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
    $token = [System.IO.Path]::GetFileName($root).Replace('simsuite-desktop-smoke-', '')
    $appData = Join-Path $root 'appdata'
    $downloads = Join-Path $root 'downloads'
    $mods = Join-Path $root 'Mods'
    $installedMccc = Join-Path $mods 'MCCC'
    $installedXml = Join-Path $mods 'XML Injector'
    $installedS4cl = Join-Path $mods 'S4CL'
    $installedLot51 = Join-Path $mods 'Lot51 Core Library'
    $installedToolbox = Join-Path $mods 'Lumpinou Toolbox'
    $installedSmartCore = Join-Path $mods 'Smart Core Script'
    $incomingRoot = Join-Path $root 'incoming-mccc'
    $blockedRoot = Join-Path $root 'blocked-mccc'
    $xmlSameRoot = Join-Path $root 'incoming-xml-same'
    $xmlOlderRoot = Join-Path $root 'incoming-xml-older'
    $s4clSameRoot = Join-Path $root 'incoming-s4cl-same'
    $s4clOlderRoot = Join-Path $root 'incoming-s4cl-older'
    $lot51SameRoot = Join-Path $root 'incoming-lot51-same'
    $lot51OlderRoot = Join-Path $root 'incoming-lot51-older'
    $toolboxSameRoot = Join-Path $root 'incoming-toolbox-same'
    $toolboxOlderRoot = Join-Path $root 'incoming-toolbox-older'
    $smartCoreSameRoot = Join-Path $root 'incoming-smart-core-same'
    $smartCoreOlderRoot = Join-Path $root 'incoming-smart-core-older'
    $specialItem = "MCCC_Update_Test_$token" 
    $blockedItem = "MCCC_Partial_Blocked_Test_$token"
    $xmlSameItem = "XML_Injector_Same_Test_$token"
    $xmlOlderItem = "XML_Injector_Older_Test_$token"
    $s4clSameItem = "S4CL_Same_Test_$token"
    $s4clOlderItem = "S4CL_Older_Test_$token"
    $lot51SameItem = "Lot51_Core_Same_Test_$token"
    $lot51OlderItem = "Lot51_Core_Older_Test_$token"
    $toolboxSameItem = "Toolbox_Same_Test_$token"
    $toolboxOlderItem = "Toolbox_Older_Test_$token"
    $smartCoreSameItem = "Smart_Core_Same_Test_$token"
    $smartCoreOlderItem = "Smart_Core_Older_Test_$token"

    foreach ($path in @(
        $appData,
        $downloads,
        $installedMccc,
        $installedXml,
        $installedS4cl,
        $installedLot51,
        $installedToolbox,
        $installedSmartCore,
        $incomingRoot,
        $blockedRoot,
        $xmlSameRoot,
        $xmlOlderRoot,
        $s4clSameRoot,
        $s4clOlderRoot,
        $lot51SameRoot,
        $lot51OlderRoot,
        $toolboxSameRoot,
        $toolboxOlderRoot,
        $smartCoreSameRoot,
        $smartCoreOlderRoot
    )) {
        New-Item -ItemType Directory -Force -Path $path | Out-Null
    }

    New-SmokeTs4script -Path (Join-Path $installedMccc 'mc_cmd_center.ts4script') -Version '2025.9.0'
    Write-SmokePackage -Path (Join-Path $installedMccc 'mc_cmd_center.package') -Content 'installed mccc package'
    Write-SmokePackage -Path (Join-Path $installedMccc 'mc_woohoo.package') -Content 'installed woohoo package'
    Write-SmokePackage -Path (Join-Path $installedMccc 'mc_settings.cfg') -Content 'setting=true'

    New-SmokeTs4script -Path (Join-Path $incomingRoot 'mc_cmd_center.ts4script') -Version '2026.1.1'
    Write-SmokePackage -Path (Join-Path $incomingRoot 'mc_cmd_center.package') -Content 'incoming mccc package'
    Write-SmokePackage -Path (Join-Path $incomingRoot 'mc_woohoo.package') -Content 'incoming woohoo package'
    New-SmokeZip -SourceRoot $incomingRoot -ZipPath (Join-Path $downloads "$specialItem.zip")

    Write-SmokePackage -Path (Join-Path $blockedRoot 'mc_woohoo.package') -Content 'partial woohoo only'
    New-SmokeZip -SourceRoot $blockedRoot -ZipPath (Join-Path $downloads "$blockedItem.zip")

    New-SmokeTs4script -Path (Join-Path $installedXml 'XmlInjector_Script_v4_0.ts4script') -Version '4.0' -Marker 'XML Injector version'
    New-SmokeTs4script -Path (Join-Path $xmlSameRoot 'XmlInjector_Script_v4_0.ts4script') -Version '4.0' -Marker 'XML Injector version'
    New-SmokeZip -SourceRoot $xmlSameRoot -ZipPath (Join-Path $downloads "$xmlSameItem.zip")

    New-SmokeTs4script -Path (Join-Path $xmlOlderRoot 'XmlInjector_Script_v3_0.ts4script') -Version '3.0' -Marker 'XML Injector version'
    New-SmokeZip -SourceRoot $xmlOlderRoot -ZipPath (Join-Path $downloads "$xmlOlderItem.zip")

    New-SmokeTs4script -Path (Join-Path $installedS4cl 'S4CL.ts4script') -Version '2.9.0' -Marker 'S4CL version'
    New-SmokeTs4script -Path (Join-Path $s4clSameRoot 'S4CL.ts4script') -Version '2.9.0' -Marker 'S4CL version'
    New-SmokeZip -SourceRoot $s4clSameRoot -ZipPath (Join-Path $downloads "$s4clSameItem.zip")

    New-SmokeTs4script -Path (Join-Path $s4clOlderRoot 'S4CL.ts4script') -Version '2.8.0' -Marker 'S4CL version'
    New-SmokeZip -SourceRoot $s4clOlderRoot -ZipPath (Join-Path $downloads "$s4clOlderItem.zip")

    New-SmokeTs4script -Path (Join-Path $installedLot51 'lot51_core.ts4script') -Version '1.41' -Marker 'Lot 51 Core Library version'
    New-SmokeTs4script -Path (Join-Path $lot51SameRoot 'lot51_core.ts4script') -Version '1.41' -Marker 'Lot 51 Core Library version'
    New-SmokeZip -SourceRoot $lot51SameRoot -ZipPath (Join-Path $downloads "$lot51SameItem.zip")

    New-SmokeTs4script -Path (Join-Path $lot51OlderRoot 'lot51_core.ts4script') -Version '1.40' -Marker 'Lot 51 Core Library version'
    New-SmokeZip -SourceRoot $lot51OlderRoot -ZipPath (Join-Path $downloads "$lot51OlderItem.zip")

    New-SmokeTs4script -Path (Join-Path $installedToolbox 'lumpinou_toolbox.ts4script') -Version '1.8.0' -Marker 'Lumpinou Toolbox version'
    Write-SmokePackage -Path (Join-Path $installedToolbox 'Lumpinou_Toolbox.package') -Content 'lumpinou toolbox package v1.8.0'
    New-SmokeTs4script -Path (Join-Path $toolboxSameRoot 'lumpinou_toolbox.ts4script') -Version '1.8.0' -Marker 'Lumpinou Toolbox version'
    Write-SmokePackage -Path (Join-Path $toolboxSameRoot 'Lumpinou_Toolbox.package') -Content 'lumpinou toolbox package v1.8.0'
    New-SmokeZip -SourceRoot $toolboxSameRoot -ZipPath (Join-Path $downloads "$toolboxSameItem.zip")

    New-SmokeTs4script -Path (Join-Path $toolboxOlderRoot 'lumpinou_toolbox.ts4script') -Version '1.7.0' -Marker 'Lumpinou Toolbox version'
    Write-SmokePackage -Path (Join-Path $toolboxOlderRoot 'Lumpinou_Toolbox.package') -Content 'incoming older lumpinou toolbox package'
    New-SmokeZip -SourceRoot $toolboxOlderRoot -ZipPath (Join-Path $downloads "$toolboxOlderItem.zip")

    New-SmokeTs4script -Path (Join-Path $installedSmartCore 'SmartCoreScript.ts4script') -Version '2.9.0' -Marker 'Smart Core Script version'
    New-SmokeTs4script -Path (Join-Path $smartCoreSameRoot 'SmartCoreScript.ts4script') -Version '2.9.0' -Marker 'Smart Core Script version'
    New-SmokeZip -SourceRoot $smartCoreSameRoot -ZipPath (Join-Path $downloads "$smartCoreSameItem.zip")

    New-SmokeTs4script -Path (Join-Path $smartCoreOlderRoot 'SmartCoreScript.ts4script') -Version '2.8.0' -Marker 'Smart Core Script version'
    New-SmokeZip -SourceRoot $smartCoreOlderRoot -ZipPath (Join-Path $downloads "$smartCoreOlderItem.zip")

    return @{
        Root = $root
        AppData = $appData
        Downloads = $downloads
        Mods = $mods
        SpecialItem = $specialItem
        BlockedItem = $blockedItem
        XmlSameItem = $xmlSameItem
        XmlOlderItem = $xmlOlderItem
        S4clSameItem = $s4clSameItem
        S4clOlderItem = $s4clOlderItem
        Lot51SameItem = $lot51SameItem
        Lot51OlderItem = $lot51OlderItem
        ToolboxSameItem = $toolboxSameItem
        ToolboxOlderItem = $toolboxOlderItem
        SmartCoreSameItem = $smartCoreSameItem
        SmartCoreOlderItem = $smartCoreOlderItem
    }
}

function Stop-SmokeProcesses {
    foreach ($name in @('tauri-driver', 'msedgedriver', 'simsuite', 'SimSuite')) {
        try {
            Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force
        } catch {
        }
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

if ($CleanSmokeProcesses) {
    Stop-SmokeProcesses
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
$launcherExitCode = $null
for ($attempt = 0; $attempt -lt 40; $attempt++) {
    Start-Sleep -Milliseconds 500
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $statusUrl | Out-Null
        $ready = $true
        break
    } catch {
    }

    if ($process.HasExited) {
        $launcherExitCode = $process.ExitCode
        if ($launcherExitCode -ne 0) {
            Write-Error "tauri-driver exited early with code $launcherExitCode."
            exit 1
        }
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

$sessionDirectory = Split-Path -Parent $SessionFile
if ($sessionDirectory) {
    New-Item -ItemType Directory -Force -Path $sessionDirectory | Out-Null
}

$session = @{
    port = $Port
    statusUrl = $statusUrl
    tauriDriverPid = if ($process.HasExited) { $null } else { $process.Id }
    tauriDriverExitCode = $launcherExitCode
    fixture = if ($fixture) {
        @{
            root = $fixture.Root
            appData = $fixture.AppData
            downloads = $fixture.Downloads
            mods = $fixture.Mods
            specialItem = $fixture.SpecialItem
            blockedItem = $fixture.BlockedItem
                xmlSameItem = $fixture.XmlSameItem
                xmlOlderItem = $fixture.XmlOlderItem
                s4clSameItem = $fixture.S4clSameItem
                s4clOlderItem = $fixture.S4clOlderItem
                lot51SameItem = $fixture.Lot51SameItem
                lot51OlderItem = $fixture.Lot51OlderItem
                toolboxSameItem = $fixture.ToolboxSameItem
                toolboxOlderItem = $fixture.ToolboxOlderItem
                smartCoreSameItem = $fixture.SmartCoreSameItem
                smartCoreOlderItem = $fixture.SmartCoreOlderItem
            }
        } else {
            $null
        }
}
$session | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 -Path $SessionFile

if ($fixture) {
    Write-Output "TAURI_DRIVER_FIXTURES root=$($fixture.Root) downloads=$($fixture.Downloads) mods=$($fixture.Mods)"
}
Write-Output "TAURI_DRIVER_SESSION file=$SessionFile"
Write-Output "TAURI_DRIVER_READY pid=$($process.Id) url=$statusUrl"
