#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Start (or restart) Docker Desktop on a remote Windows machine.

.DESCRIPTION
    Checks if Docker Desktop is already responding. If it is, exits immediately.
    If it is not responding:
      - If Docker Desktop processes exist (hung/crashed), kills them first.
      - Then launches Docker Desktop fresh.
      - Polls until the Linux engine (desktop-linux context) is ready.

    Use -ForceRestart to skip the initial check and always kill+relaunch.

.PARAMETER RemoteHost
    SSH target for the remote build machine, e.g. joel@192.168.1.110.
    If omitted, runs on the LOCAL machine.

.PARAMETER WaitSeconds
    Seconds to wait after launching before starting to poll (default: 70).

.PARAMETER TimeoutSeconds
    Total seconds to poll for Docker readiness (default: 180).

.PARAMETER ForceRestart
    Always kill and relaunch Docker Desktop, even if it appears to be running.

.EXAMPLE
    # Only restart if not already running/responsive:
    .\scripts\restart-remote-docker.ps1 -RemoteHost joel@192.168.1.110

    # Force a full kill+restart regardless:
    .\scripts\restart-remote-docker.ps1 -RemoteHost joel@192.168.1.110 -ForceRestart
#>
param(
    [string]$RemoteHost      = "",
    [int]   $WaitSeconds     = 90,
    [int]   $TimeoutSeconds  = 300,
    [switch]$ForceRestart
)

$ErrorActionPreference = "Stop"

# ─── Helper: test if remote Docker responds via SSH tunnel ───────────────────

function Test-DockerReady ([string]$SshHost) {
    if ($SshHost) { $env:DOCKER_HOST = "ssh://$SshHost" }
    try {
        $out = docker info --format "{{.ServerVersion}}" 2>&1
        return ($LASTEXITCODE -eq 0 -and ($out -notmatch "error|Error|unable"))
    }
    catch { return $false }
    finally {
        if ($SshHost) { Remove-Item Env:\DOCKER_HOST -ErrorAction SilentlyContinue }
    }
}

# ─── Helper: poll until Docker is ready ─────────────────────────────────────

function Wait-DockerReady ([string]$SshHost, [int]$InitialWait, [int]$Timeout) {
    if ($InitialWait -gt 0) {
        Write-Host "  Waiting ${InitialWait}s for Docker Desktop to initialise..." -ForegroundColor Yellow
        Start-Sleep $InitialWait
    }

    Write-Host "  Polling for readiness (timeout: ${Timeout}s)..." -ForegroundColor Yellow
    $deadline = (Get-Date).AddSeconds($Timeout)
    $attempt  = 0

    while ((Get-Date) -lt $deadline) {
        $attempt++
        Write-Host "    Attempt $attempt ..." -NoNewline
        if (Test-DockerReady $SshHost) {
            Write-Host " ready!" -ForegroundColor Green
            return $true
        }
        Write-Host " not yet"
        Start-Sleep 5
    }
    return $false
}

# ─── Lines run on the remote to launch Docker Desktop ───────────────────────

$launchLines = @(
    '# Prefer schtasks (runs in user GUI session = can start WSL2/Docker Desktop)',
    '# Fall back to Start-Process if the task has not been registered yet.',
    '$taskExists = schtasks /query /tn "LaserTargets-StartDocker" 2>&1',
    'if ($LASTEXITCODE -eq 0) {',
    '    Write-Host "Starting Docker Desktop via scheduled task (Session 1)..."',
    '    schtasks /run /tn "LaserTargets-StartDocker" | Out-Null',
    '} else {',
    '    Write-Host "Scheduled task not found -- falling back to Start-Process."',
    '    Write-Host "Run setup-remote-docker-task.ps1 once to enable SSH-based startup."',
    '    $exe = Join-Path $env:ProgramFiles "Docker\Docker\Docker Desktop.exe"',
    '    if (-not (Test-Path $exe)) { Write-Error "Docker Desktop not found at: $exe"; exit 1 }',
    '    $proc = Get-Process "Docker Desktop" -ErrorAction SilentlyContinue',
    '    if ($proc) {',
    '        Write-Host "Docker Desktop already running (pid $($proc.Id -join '','')) -- not killing."',
    '    } else {',
    '        Start-Process -FilePath $exe',
    '    }',
    '}',
    'docker context use desktop-linux | Out-Null'
)

$launchLinesNoKill = $launchLines  # same: never kill if using schtasks

# ─── Main logic ─────────────────────────────────────────────────────────────

$target = if ($RemoteHost) { $RemoteHost } else { "this machine" }
Write-Host "Checking Docker Desktop on $target ..." -ForegroundColor Cyan

# Step 1: if already responding and not forcing, we're done
if (-not $ForceRestart -and (Test-DockerReady $RemoteHost)) {
    Write-Host "  Docker Desktop is already running and ready. Nothing to do." -ForegroundColor Green
    exit 0
}

if ($ForceRestart) {
    Write-Host "  -ForceRestart specified -- killing and relaunching..." -ForegroundColor Yellow
    $scriptContent = $launchLines
} else {
    Write-Host "  Docker Desktop not responding -- starting (only kills if process exists)..." -ForegroundColor Yellow
    $scriptContent = $launchLinesNoKill
}

# Step 2: run the launch script locally or on the remote
if ($RemoteHost) {
    $localTemp  = [System.IO.Path]::ChangeExtension([System.IO.Path]::GetTempFileName(), ".ps1")
    $remotePath = "C:\tmp\__launch_docker.ps1"

    $scriptContent | Set-Content -Path $localTemp -Encoding UTF8

    try {
        ssh $RemoteHost "powershell -Command New-Item -ItemType Directory -Force -Path C:\tmp | Out-Null" 2>&1 | Out-Null
        scp $localTemp "${RemoteHost}:${remotePath}"
        if ($LASTEXITCODE -ne 0) { throw "scp failed" }
        ssh $RemoteHost "powershell -ExecutionPolicy Bypass -File $remotePath"
        ssh $RemoteHost "powershell -Command Remove-Item -Force $remotePath" 2>&1 | Out-Null
    }
    finally {
        Remove-Item $localTemp -ErrorAction SilentlyContinue
    }
}
else {
    $scriptContent | ForEach-Object { Invoke-Expression $_ }
}

# Step 3: poll until ready
$ready = Wait-DockerReady -SshHost $RemoteHost -InitialWait $WaitSeconds -Timeout $TimeoutSeconds

if (-not $ready) {
    Write-Error "Docker Desktop on $target did not become ready within ${TimeoutSeconds}s."
}

Write-Host ""
Write-Host "Docker Desktop is ready on $target." -ForegroundColor Green
if ($RemoteHost) {
    Write-Host "You can now run:" -ForegroundColor Cyan
    Write-Host "  .\scripts\docker-build-rpi4-remote.ps1 -RemoteHost $RemoteHost"
}
