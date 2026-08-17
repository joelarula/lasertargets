#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Append-Mode Progress Monitor for Remote Docker Builds on Windows Server host.

.DESCRIPTION
    Polls the remote Docker host via SSH and appends timestamped progress log lines
    showing elapsed time, active build layers, process CPU seconds, and memory footprint.

.PARAMETER TargetHost
    The SSH remote host (e.g. joel@192.168.1.110). Defaults to joel@192.168.1.110.

.PARAMETER RefreshInterval
    Refresh interval in seconds. Defaults to 5.

.EXAMPLE
    .\scripts\watch-remote-build.ps1
    .\scripts\watch-remote-build.ps1 -TargetHost joel@192.168.1.110 -RefreshInterval 3
#>
param(
    [string]$TargetHost = "joel@192.168.1.110",
    [int]$RefreshInterval = 5
)

$ErrorActionPreference = "SilentlyContinue"
$startTime = Get-Date

Write-Host "=== LaserTargets Remote Build Monitor (Append Mode) ===" -ForegroundColor Cyan
Write-Host "Target Host: $TargetHost" -ForegroundColor Gray
Write-Host "Press Ctrl+C to stop monitoring.`n" -ForegroundColor DarkGray

$remoteProcCmd = "Get-Process com.docker.build, wsl, rustc -ErrorAction SilentlyContinue | Select-Object -First 3 | ForEach-Object { '{0} CPU:{1:N1}s Mem:{2:N0}MB' -f `$_.Name, `$_.CPU, (`$_.WorkingSet64/1MB) }"
$remoteBuilderCmd = "docker builder du | Select-String 'false' | Measure-Object | Select-Object -ExpandProperty Count"

while ($true) {
    $now = Get-Date -Format "HH:mm:ss"
    $elapsed = (Get-Date) - $startTime
    $elapsedStr = "{0:D2}:{1:D2}:{2:D2}" -f $elapsed.Hours, $elapsed.Minutes, $elapsed.Seconds

    $procOutput = ssh -o ConnectTimeout=3 $TargetHost $remoteProcCmd
    $activeLayers = ssh -o ConnectTimeout=3 $TargetHost $remoteBuilderCmd

    if (-not $procOutput) {
        $procSummary = "Idle / Finalizing"
    } else {
        $procSummary = ($procOutput -join " | ")
    }

    if (-not $activeLayers) { $activeLayers = 0 }

    Write-Host "[$now] Elapsed: $elapsedStr | Active Build Layers: $activeLayers | Processes: $procSummary" -ForegroundColor Yellow

    Start-Sleep -Seconds $RefreshInterval
}
