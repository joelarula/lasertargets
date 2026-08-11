#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Deploy the LaserTargets server to a Raspberry Pi natively from Windows.

.DESCRIPTION
    Stops the existing systemd service on the Pi, copies the cross-compiled
    binary and Helios library, updates the service and logrotate files,
    and restarts the service.

.PARAMETER TargetHost
    The Raspberry Pi host (e.g., lasertargets.local or lasertargets@192.168.1.100).
    Defaults to lasertargets.local.

.EXAMPLE
    .\scripts\deploy-pi.ps1
    .\scripts\deploy-pi.ps1 -TargetHost lasertargets@192.168.1.50
#>
param(
    [string]$TargetHost = "lasertargets.local"
)

$ErrorActionPreference = "Stop"

# Default to lasertargets@ if no user specified
if ($TargetHost -notmatch "@") {
    $TargetHost = "lasertargets@$TargetHost"
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir "..")
$DistDir = Join-Path $ProjectRoot "dist\pi"
$ServerBinary = Join-Path $DistDir "server"
$HeliosLibrary = Join-Path $DistDir "libHeliosLaserDAC.so"

Write-Host "=== Deploying LaserTargets to $TargetHost ===" -ForegroundColor Cyan

# Verify build artifacts exist
if (-not (Test-Path $ServerBinary)) {
    Write-Error "Build artifact not found: $ServerBinary. Run .\scripts\docker-build-rpi4-remote.ps1 first."
}

# Stop the service if it's running
Write-Host "--- Stopping existing service (if running) ---" -ForegroundColor Yellow
$null = ssh $TargetHost "sudo systemctl stop lasertargets-server 2>/dev/null || true"

# Create directories on the Pi
Write-Host "--- Ensuring directory structure ---" -ForegroundColor Yellow
$null = ssh $TargetHost "sudo mkdir -p /opt/lasertargets/lib && sudo chown -R lasertargets:lasertargets /opt/lasertargets"

# Deploy binary
Write-Host "--- Deploying server binary ---" -ForegroundColor Yellow
$null = ssh $TargetHost "rm -f /opt/lasertargets/server"
scp $ServerBinary "${TargetHost}:/opt/lasertargets/server"
$null = ssh $TargetHost "chmod +x /opt/lasertargets/server"

# Deploy dac-test (hardware test tool — run manually)
$DacTestBinary = Join-Path $DistDir "dac-test"
if (Test-Path $DacTestBinary) {
    Write-Host "--- Deploying dac-test binary ---" -ForegroundColor Yellow
    $null = ssh $TargetHost "rm -f /opt/lasertargets/dac-test"
    scp $DacTestBinary "${TargetHost}:/opt/lasertargets/dac-test"
    $null = ssh $TargetHost "chmod +x /opt/lasertargets/dac-test"
} else {
    Write-Host "--- Skipping dac-test (not built yet) ---" -ForegroundColor DarkGray
}

# Deploy shared library (if available)
if (Test-Path $HeliosLibrary) {
    Write-Host "--- Deploying Helios DAC library ---" -ForegroundColor Yellow
    scp $HeliosLibrary "${TargetHost}:/opt/lasertargets/lib/"
} else {
    Write-Host "--- Skipping Helios DAC library (not built) ---" -ForegroundColor DarkGray
}

# Deploy systemd service file
Write-Host "--- Installing systemd service ---" -ForegroundColor Yellow
scp (Join-Path $ProjectRoot "deploy\lasertargets-server.service") "${TargetHost}:/tmp/lasertargets-server.service"
$null = ssh $TargetHost "sudo mv /tmp/lasertargets-server.service /etc/systemd/system/ && sudo systemctl daemon-reload"

# Deploy logrotate configuration
$LogrotateConfig = Join-Path $ProjectRoot "deploy\lasertargets-logrotate"
if (Test-Path $LogrotateConfig) {
    Write-Host "--- Installing logrotate configuration ---" -ForegroundColor Yellow
    scp $LogrotateConfig "${TargetHost}:/tmp/lasertargets-logrotate"
    $null = ssh $TargetHost "sudo mv /tmp/lasertargets-logrotate /etc/logrotate.d/lasertargets && sudo chown root:root /etc/logrotate.d/lasertargets && sudo chmod 644 /etc/logrotate.d/lasertargets"
}

# Enable and start the service
Write-Host "--- Starting service ---" -ForegroundColor Yellow
$null = ssh $TargetHost "sudo systemctl enable lasertargets-server && sudo systemctl start lasertargets-server"

# Check status
Write-Host ""
Write-Host "--- Service status ---" -ForegroundColor Yellow
ssh $TargetHost "sudo systemctl status lasertargets-server --no-pager"

Write-Host ""
Write-Host "=== Deployment complete ===" -ForegroundColor Green
Write-Host "Useful commands:"
Write-Host "  ssh $TargetHost 'sudo systemctl status lasertargets-server'"
Write-Host "  ssh $TargetHost 'sudo journalctl -u lasertargets-server -f'"
Write-Host "  ssh $TargetHost 'sudo systemctl restart lasertargets-server'"
