param(
    [int]$Port = 1420,
    [switch]$Quiet
)

$ErrorActionPreference = 'Stop'

function Get-ListeningProcessIds {
    param([int]$LocalPort)

    @(Get-NetTCPConnection -State Listen -LocalPort $LocalPort -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty OwningProcess -Unique)
}

function Get-ProcessCommandLine {
    param([int]$ProcessId)

    try {
        return (Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction Stop).CommandLine
    } catch {
        return $null
    }
}

function Stop-ProcessTree {
    param([int]$ProcessId)

    & taskkill /PID $ProcessId /T /F | Out-Null
}

$processIds = Get-ListeningProcessIds -LocalPort $Port

if (-not $processIds -or $processIds.Count -eq 0) {
    if (-not $Quiet) {
        Write-Output "SIMSUITE_DEV_PORT status=free port=$Port"
    }
    return
}

$blocked = @()

foreach ($processId in $processIds) {
    $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
    if (-not $process) {
        continue
    }

    $commandLine = Get-ProcessCommandLine -ProcessId $processId
    $looksLikeVite = $process.ProcessName -ieq 'node' -and $commandLine -and ($commandLine -match 'vite')

    if ($looksLikeVite) {
        if (-not $Quiet) {
            Write-Output "SIMSUITE_DEV_PORT action=stop port=$Port pid=$processId name=$($process.ProcessName)"
        }
        Stop-ProcessTree -ProcessId $processId
        continue
    }

    $blocked += [pscustomobject]@{
        Id = $processId
        Name = $process.ProcessName
        CommandLine = $commandLine
    }
}

if ($blocked.Count -gt 0) {
    $details = $blocked | ForEach-Object {
        if ($_.CommandLine) {
            "$($_.Name) (PID $($_.Id)): $($_.CommandLine)"
        } else {
            "$($_.Name) (PID $($_.Id))"
        }
    }
    throw "Port $Port is already being used by another process. SimSuite did not stop it automatically. $($details -join '; ')"
}

Start-Sleep -Milliseconds 300

$remaining = Get-ListeningProcessIds -LocalPort $Port
if ($remaining -and $remaining.Count -gt 0) {
    throw "Port $Port is still busy after cleanup. Please close the remaining process and try again."
}

if (-not $Quiet) {
    Write-Output "SIMSUITE_DEV_PORT status=cleared port=$Port"
}
