#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Unified manager for remote Windows Docker build workstations.

.DESCRIPTION
    Consolidates remote Docker Desktop health checks, restarts, scheduled task setup,
    and build compilation monitoring.

.PARAMETER Action
    The action to execute:
      - status     : Test if remote Docker daemon is responsive.
      - start      : Start remote Docker Desktop via scheduled task if not already running.
      - restart    : Kill any hung remote Docker processes and launch fresh.
      - setup-task : One-time registration of the StartDocker scheduled task on the remote host.
      - watch      : Live monitor for active container compilation threads and CPU usage.

.PARAMETER RemoteHost
    SSH target for the remote build machine (e.g. joel@192.168.1.110).

.PARAMETER RemoteUser
    Windows user account on the remote host (default: 'joel').

.PARAMETER TimeoutSeconds
    Timeout for waiting for Docker daemon readiness (default: 180).

.EXAMPLE
    .\scripts\remote-docker.ps1 status -RemoteHost joel@192.168.1.110
    .\scripts\remote-docker.ps1 restart -RemoteHost joel@192.168.1.110
    .\scripts\remote-docker.ps1 watch -RemoteHost joel@192.168.1.110
#>
param(
    [Parameter(Position = 0)]
    [ValidateSet("status", "start", "restart", "setup-task", "watch")]
    [string]$Action = "status",

    [Parameter(Position = 1)]
    [string]$RemoteHost = "joel@192.168.1.110",

    [string]$RemoteUser = "joel",

    [int]$TimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

function Test-RemoteDockerReady {
    try {
        $out = ssh -o ConnectTimeout=5 $RemoteHost "docker info --format '{{.ServerVersion}}'" 2>&1
        return ($LASTEXITCODE -eq 0 -and ($out -notmatch "error|Error|unable"))
    } catch {
        return $false
    }
}

function Wait-DockerReady ([int]$Timeout) {
    Write-Host "Polling remote Docker daemon until ready (up to ${Timeout}s)..." -ForegroundColor Cyan
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    while ($stopwatch.Elapsed.TotalSeconds -lt $Timeout) {
        if (Test-RemoteDockerReady) {
            Write-Host "  [READY] Docker Desktop is active and responding!" -ForegroundColor Green
            return $true
        }
        Write-Host "  ... waiting for engine ($([int]$stopwatch.Elapsed.TotalSeconds)s)" -ForegroundColor Gray
        Start-Sleep -Seconds 5
    }
    Write-Host "  [TIMEOUT] Docker Desktop did not become ready within ${Timeout}s." -ForegroundColor Red
    return $false
}

# ==============================================================================
# ACTION: status
# ==============================================================================
if ($Action -eq "status") {
    Write-Host "Checking remote Docker status at $RemoteHost..." -ForegroundColor Cyan
    if (Test-RemoteDockerReady) {
        Write-Host "  [PASS] Remote Docker Desktop is online and responding." -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Remote Docker Desktop is NOT responding via SSH." -ForegroundColor Red
    }
    exit 0
}

# ==============================================================================
# ACTION: setup-task
# ==============================================================================
if ($Action -eq "setup-task") {
    Write-Host "Registering Docker Desktop scheduled task on $RemoteHost..." -ForegroundColor Cyan

    $taskScript = @(
        'schtasks /delete /tn "LaserTargets-StartDocker" /f 2>&1 | Out-Null',
        "schtasks /create /tn `"LaserTargets-StartDocker`" /tr `"`"C:\Program Files\Docker\Docker\Docker Desktop.exe`"`" /sc once /st 00:00 /ru `"$RemoteUser`" /it /rl HIGHEST /f 2>&1",
        'if ($LASTEXITCODE -eq 0) { Write-Host "Task registered successfully." } else { Write-Error "Failed to register task." }'
    ) -join "`n"

    $encoded = [Convert]::ToBase64String([System.Text.Encoding]::Unicode.GetBytes($taskScript))
    ssh $RemoteHost "powershell.exe -NoProfile -EncodedCommand $encoded"
    exit 0
}

# ==============================================================================
# ACTION: start or restart
# ==============================================================================
if ($Action -eq "start" -or $Action -eq "restart") {
    if ($Action -eq "start" -and (Test-RemoteDockerReady)) {
        Write-Host "Docker Desktop is already running and ready on $RemoteHost." -ForegroundColor Green
        exit 0
    }

    if ($Action -eq "restart") {
        Write-Host "Stopping any running Docker processes on $RemoteHost..." -ForegroundColor Yellow
        $killCmd = "Get-Process *docker*, com.docker.* -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue"
        $encKill = [Convert]::ToBase64String([System.Text.Encoding]::Unicode.GetBytes($killCmd))
        $null = ssh $RemoteHost "powershell.exe -NoProfile -EncodedCommand $encKill"
        Start-Sleep -Seconds 3
    }

    Write-Host "Triggering Docker Desktop launch via scheduled task on $RemoteHost..." -ForegroundColor Yellow
    $null = ssh $RemoteHost 'schtasks /run /tn "LaserTargets-StartDocker" 2>&1'

    if (-not (Wait-DockerReady -Timeout $TimeoutSeconds)) {
        exit 1
    }
    exit 0
}

# ==============================================================================
# ACTION: watch
# ==============================================================================
if ($Action -eq "watch") {
    Write-Host "=== LaserTargets Live Remote Compiler Monitor ===" -ForegroundColor Cyan
    Write-Host "Target Host: $RemoteHost (Press Ctrl+C to stop)`n" -ForegroundColor DarkGray
    $startTime = Get-Date

    while ($true) {
        $now = Get-Date -Format "HH:mm:ss"
        $elapsed = (Get-Date) - $startTime
        $elapsedStr = "{0:D2}:{1:D2}:{2:D2}" -f $elapsed.Hours, $elapsed.Minutes, $elapsed.Seconds

        $activeProc = ssh -o ConnectTimeout=3 $RemoteHost "wsl -d docker-desktop sh -c 'ps aux | grep -E `"rustc|cargo|aarch64-linux-gnu-gcc`" | grep -v grep | grep -v `"wsl-bootstrap`"'" 2>$null

        if ($activeProc) {
            if ($activeProc -match "rustc.*--crate-name\s+([a-zA-Z0-9_\-]+)") {
                $crateName = $Matches[1]
                Write-Host "[$now +$elapsedStr] Compiling crate: $crateName" -ForegroundColor Green
            } else {
                Write-Host "[$now +$elapsedStr] Active: Container toolchain compiling..." -ForegroundColor Yellow
            }
        } else {
            Write-Host "[$now +$elapsedStr] Idle / Linking / Waiting..." -ForegroundColor DarkGray
        }
        Start-Sleep -Seconds 3
    }
}

