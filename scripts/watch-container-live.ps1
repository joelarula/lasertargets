#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Live Container Compilation Monitor for Remote Docker Builds.

.DESCRIPTION
    Polls active running processes inside docker-desktop, ignoring static toolchain files.

.PARAMETER TargetHost
    The SSH remote host (e.g. joel@192.168.1.110). Defaults to joel@192.168.1.110.

.PARAMETER RefreshInterval
    Refresh interval in seconds. Defaults to 3.

.EXAMPLE
    .\scripts\watch-container-live.ps1
    .\scripts\watch-container-live.ps1 -TargetHost joel@192.168.1.110
#>
param(
    [string]$TargetHost = "joel@192.168.1.110",
    [int]$RefreshInterval = 3
)

$ErrorActionPreference = "SilentlyContinue"
$startTime = Get-Date

Write-Host "=== LaserTargets Live Container Compiler Monitor ===" -ForegroundColor Cyan
Write-Host "Target Host: $TargetHost" -ForegroundColor Gray
Write-Host "Press Ctrl+C to stop monitoring.`n" -ForegroundColor DarkGray

while ($true) {
    $now = Get-Date -Format "HH:mm:ss"
    $elapsed = (Get-Date) - $startTime
    $elapsedStr = "{0:D2}:{1:D2}:{2:D2}" -f $elapsed.Hours, $elapsed.Minutes, $elapsed.Seconds

    # Query active container process threads inside docker-desktop
    $activeProc = ssh -o ConnectTimeout=3 $TargetHost "wsl -d docker-desktop sh -c 'ps aux | grep -E `"rustc|cargo|aarch64-linux-gnu-gcc`" | grep -v grep | grep -v `"wsl-bootstrap`"'"

    if ($activeProc) {
        if ($activeProc -match "rustc.*--crate-name\s+([a-zA-Z0-9_\-]+)") {
            $crateName = $Matches[1]
            Write-Host "[$now] Elapsed: $elapsedStr | Compiling Crate: [$crateName] (rustc active)" -ForegroundColor Green
        } elseif ($activeProc -match "aarch64-linux-gnu-gcc") {
            Write-Host "[$now] Elapsed: $elapsedStr | Linker Active: [aarch64-linux-gnu-gcc] (Linking ARM64 executable)" -ForegroundColor Yellow
        } else {
            Write-Host "[$now] Elapsed: $elapsedStr | Active Work: $activeProc" -ForegroundColor Cyan
        }
    } else {
        Write-Host "[$now] Elapsed: $elapsedStr | Container Status: Complete / Standby" -ForegroundColor DarkGray
    }

    Start-Sleep -Seconds $RefreshInterval
}
