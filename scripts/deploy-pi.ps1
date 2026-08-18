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
$null = ssh $TargetHost "sudo mkdir -p /opt/lasertargets/lib /opt/lasertargets/stats && sudo chown -R lasertargets:lasertargets /opt/lasertargets"

# Deploy binary
$ServerSizeMB = [math]::Round((Get-Item $ServerBinary).Length / 1MB, 1)
Write-Host "--- Deploying server binary ($ServerSizeMB MB, transferring...) ---" -ForegroundColor Yellow
$null = ssh $TargetHost "rm -f /opt/lasertargets/server"
scp $ServerBinary "${TargetHost}:/opt/lasertargets/server"
$null = ssh $TargetHost "chmod +x /opt/lasertargets/server"
# Grant the server binary the ability to set real-time thread priority (SCHED_FIFO)
# without running as root. This is needed for the DAC output thread's RT scheduling,
# and works for both systemd service runs and interactive runs via run-server-pi.ps1.
$null = ssh $TargetHost "sudo setcap cap_sys_nice+eip /opt/lasertargets/server 2>/dev/null || true"


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
    scp $HeliosLibrary "${TargetHost}:/opt/lasertargets/"
    $null = ssh $TargetHost "sudo cp /opt/lasertargets/libHeliosLaserDAC.so /usr/lib/libHeliosLaserDAC.so 2>/dev/null || true"
}

# Deploy assets folder (fonts, audio, etc.)
$AssetsDir = Join-Path $ProjectRoot "assets"
if (Test-Path $AssetsDir) {
    Write-Host "--- Deploying assets folder ---" -ForegroundColor Yellow
    scp -r $AssetsDir "${TargetHost}:/opt/lasertargets/"
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

# Ensure background service is disabled & stopped so USB DAC is completely free for manual/interactive testing
Write-Host "--- Disabling background service (releasing USB DAC) ---" -ForegroundColor Yellow
$null = ssh $TargetHost "sudo systemctl disable --now lasertargets-server 2>/dev/null; sudo killall -9 server 2>/dev/null || true"

Write-Host ""
Write-Host "=== Deployment complete ===" -ForegroundColor Green
Write-Host "Binaries & library updated at /opt/lasertargets/" -ForegroundColor Gray
Write-Host "Run interactively with logs: .\scripts\run-server-pi.ps1" -ForegroundColor Cyan
Write-Host "Or start background service:  ssh $TargetHost 'sudo systemctl start lasertargets-server'" -ForegroundColor DarkGray
Write-Host "  ssh $TargetHost 'sudo systemctl restart lasertargets-server'"
