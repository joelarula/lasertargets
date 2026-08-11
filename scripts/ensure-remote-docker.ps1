#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Ensure Docker Desktop is running and ready on a remote Windows machine.

.DESCRIPTION
    Checks whether the remote Docker Desktop (desktop-linux context) is
    responding via DOCKER_HOST=ssh://. If not, starts Docker Desktop on the
    remote via SSH and polls until the daemon is ready (or times out).
    Also ensures the remote context is set to desktop-linux, which is required
    for Linux/aarch64 cross-compilation.

.PARAMETER RemoteHost
    SSH target for the remote build machine, e.g. joel@192.168.1.110.

.PARAMETER TimeoutSeconds
    How long to wait for Docker Desktop to become ready (default: 90).

.EXAMPLE
    .\scripts\ensure-remote-docker.ps1 -RemoteHost joel@192.168.1.110
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$RemoteHost,

    [int]$TimeoutSeconds = 90
)

$ErrorActionPreference = "Stop"

# Helper: test if remote Docker daemon is reachable via SSH tunnel
function Test-RemoteDockerReady {
    $env:DOCKER_HOST = "ssh://$RemoteHost"
    try {
        $out = docker info --format "{{.ServerVersion}}" 2>&1
        return ($LASTEXITCODE -eq 0 -and ($out -notmatch "error|Error|unable"))
    }
    catch {
        return $false
    }
    finally {
        Remove-Item Env:\DOCKER_HOST -ErrorAction SilentlyContinue
    }
}

# Step 1: check if already up
Write-Host "Checking remote Docker at $RemoteHost ..." -ForegroundColor Cyan
if (Test-RemoteDockerReady) {
    Write-Host "  Docker Desktop is already running and ready." -ForegroundColor Green
}
else {
    Write-Host "  Docker Desktop not responding -- starting it on the remote..." -ForegroundColor Yellow

    # Step 2: write a helper script locally, SCP it to the remote, run it.
    # Using a file avoids all SSH quoting issues with spaces in the exe path.
    $localTemp = [System.IO.Path]::ChangeExtension(
        [System.IO.Path]::GetTempFileName(), ".ps1")

    # Build helper content as individual lines (avoids here-string col-0 issues)
    $helperLines = @(
        '$exe = Join-Path $env:ProgramFiles "Docker\Docker\Docker Desktop.exe"',
        'if (-not (Test-Path $exe)) { Write-Error "Docker Desktop not found at: $exe"; exit 1 }',
        '$proc = Get-Process "Docker Desktop" -ErrorAction SilentlyContinue',
        'if ($proc) {',
        '    Write-Host "Docker Desktop already running (pid $($proc.Id)) -- may still be initialising."',
        '} else {',
        '    Write-Host "Launching Docker Desktop: $exe"',
        '    Start-Process -FilePath $exe',
        '}',
        'docker context use desktop-linux 2>&1 | Out-Null',
        'Write-Host "Startup triggered."'
    )
    $helperLines | Set-Content -Path $localTemp -Encoding UTF8

    $tempRemotePath = "C:\tmp\__start_docker.ps1"

    try {
        # Ensure C:\tmp exists on the remote
        ssh $RemoteHost "powershell -Command New-Item -ItemType Directory -Force -Path C:\tmp | Out-Null" 2>&1 | Out-Null

        # Copy helper script to remote
        scp $localTemp "${RemoteHost}:${tempRemotePath}"
        if ($LASTEXITCODE -ne 0) {
            throw "scp failed: cannot copy startup helper to $RemoteHost"
        }

        # Run the helper on the remote
        ssh $RemoteHost "powershell -ExecutionPolicy Bypass -File $tempRemotePath"

        # Clean up (best effort)
        ssh $RemoteHost "powershell -Command Remove-Item -Force $tempRemotePath" 2>&1 | Out-Null
    }
    finally {
        Remove-Item $localTemp -ErrorAction SilentlyContinue
    }

    # Step 3: poll until the daemon responds
    Write-Host "  Waiting for Docker Desktop to become ready (timeout: ${TimeoutSeconds}s)..." -ForegroundColor Yellow
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $attempt  = 0
    $ready    = $false

    while ((Get-Date) -lt $deadline) {
        $attempt++
        Start-Sleep -Seconds 5
        Write-Host "    Attempt $attempt ..." -NoNewline
        if (Test-RemoteDockerReady) {
            Write-Host " ready!" -ForegroundColor Green
            $ready = $true
            break
        }
        Write-Host " not yet"
    }

    if (-not $ready) {
        Write-Host "  First start attempt timed out -- trying a full restart..." -ForegroundColor Yellow
        $scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
        & (Join-Path $scriptRoot "restart-remote-docker.ps1") -RemoteHost $RemoteHost -WaitSeconds 45 -TimeoutSeconds 120
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}

# Step 4: confirm context is desktop-linux
# Required for Linux/aarch64 builds -- NOT the Windows 'default' engine.
Write-Host "  Ensuring remote Docker context is desktop-linux ..." -ForegroundColor Cyan
ssh $RemoteHost "docker context use desktop-linux" | Out-Null
Write-Host "  Remote Docker ready: desktop-linux (Linux containers, aarch64 builds OK)." -ForegroundColor Green
