#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Run LaserTargets server interactively on the Raspberry Pi with direct terminal output.

.DESCRIPTION
    Ensures any background lasertargets-server service is stopped (releasing the USB DAC),
    executes the server binary directly on the Pi in interactive mode so you can see
    live logs in real-time.

.PARAMETER TargetHost
    The Raspberry Pi host (e.g., lasertargets.local or 192.168.1.246).
    Defaults to lasertargets.local.

.PARAMETER LogLevel
    The RUST_LOG verbosity level: info, debug, trace, warn, error.
    Defaults to 'info'.

.EXAMPLE
    .\scripts\run-server-pi.ps1
    .\scripts\run-server-pi.ps1 -LogLevel debug
    .\scripts\run-server-pi.ps1 -TargetHost 192.168.1.246 -LogLevel trace
#>
param(
    [string]$TargetHost = "lasertargets.local",

    [ValidateSet("info", "debug", "trace", "warn", "error")]
    [string]$LogLevel = "info"
)

$ErrorActionPreference = "Stop"

if ($TargetHost -notmatch "@") {
    $TargetHost = "lasertargets@$TargetHost"
}

Write-Host "=== Running LaserTargets Server Interactively ($TargetHost) ===" -ForegroundColor Cyan

try {
    Write-Host "--- Ensuring background service is stopped & freeing USB DAC ---" -ForegroundColor Yellow
    ssh $TargetHost "sudo systemctl stop lasertargets-server 2>/dev/null; sudo killall -9 server 2>/dev/null || true"
    Start-Sleep -Seconds 1

    Write-Host "--- Optimizing Pi CPU governor & USB power settings ---" -ForegroundColor Yellow
    $null = ssh $TargetHost "echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1 || true; echo -1 | sudo tee /sys/module/usbcore/parameters/autosuspend >/dev/null 2>&1 || true"

    Write-Host "--- Launching server directly (Press Ctrl+C to stop) ---" -ForegroundColor Green
    Write-Host "Log Level: RUST_LOG=$LogLevel" -ForegroundColor DarkGray
    Write-Host "--------------------------------------------------------" -ForegroundColor Gray

    ssh -t $TargetHost "cd /opt/lasertargets && sudo env LD_LIBRARY_PATH=/opt/lasertargets RUST_LOG=$LogLevel /opt/lasertargets/server"
}
finally {
    Write-Host ""
    Write-Host "--- Interactive session closed. Service left stopped. ---" -ForegroundColor Gray
}
