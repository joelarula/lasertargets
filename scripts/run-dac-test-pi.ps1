#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Deploy and run the DAC hardware test on the Raspberry Pi.

.DESCRIPTION
    Stops the lasertargets-server systemd service on the Raspberry Pi (to free up the USB DAC),
    copies the cross-compiled `dac-test` binary and Helios library, executes `dac-test`
    with the selected scenario, and restarts the server service afterwards.

.PARAMETER Scenario
    The test scenario to execute: info, blink, box, stress, sweep.
    Defaults to 'info'.

.PARAMETER Duration
    Optional duration in seconds (for scenarios like blink, box, stress, sweep).

.PARAMETER TargetHost
    The Raspberry Pi host (e.g., lasertargets.local or lasertargets@192.168.1.100).
    Defaults to lasertargets.local.

.EXAMPLE
    .\scripts\run-dac-test-pi.ps1 -Scenario info
    .\scripts\run-dac-test-pi.ps1 -Scenario box -Duration 30
    .\scripts\run-dac-test-pi.ps1 -Scenario stress -Duration 300
    .\scripts\run-dac-test-pi.ps1 -Scenario sweep
#>
param(
    [ValidateSet("all", "info", "blink", "box", "stress", "sweep", "shapes")]
    [string]$Scenario = "shapes",

    [int]$Duration = 0,

    [string]$TargetHost = "lasertargets.local"
)

$ErrorActionPreference = "Stop"

if ($TargetHost -notmatch "@") {
    $TargetHost = "lasertargets@$TargetHost"
}

$ScriptDir     = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot   = Resolve-Path (Join-Path $ScriptDir "..")
$DistDir       = Join-Path $ProjectRoot "dist\pi"
$DacTestBinary = Join-Path $DistDir "dac-test"
$HeliosLibrary = Join-Path $DistDir "libHeliosLaserDAC.so"

Write-Host "=== LaserTargets DAC Hardware Test ($TargetHost) ===" -ForegroundColor Cyan
$scenarioMsg = "Running scenario '$Scenario'"
if ($Duration -gt 0) {
    $scenarioMsg += " for $Duration s"
}
Write-Host $scenarioMsg -ForegroundColor Cyan

# 1. Verify build artifacts
if (-not (Test-Path $DacTestBinary)) {
    Write-Error "dac-test binary not found: $DacTestBinary. Run .\scripts\docker-build-rpi4-remote.ps1 first."
}

try {
    # 2. Stop server service to release DAC USB device
    Write-Host "--- Stopping lasertargets-server service (releasing USB DAC) ---" -ForegroundColor Yellow
    ssh $TargetHost "sudo systemctl stop lasertargets-server 2>/dev/null || true"

    # 3. Ensure target directory
    ssh $TargetHost "sudo mkdir -p /opt/lasertargets/lib && sudo chown -R lasertargets:lasertargets /opt/lasertargets"

    # 4. Copy dac-test binary
    Write-Host "--- Copying dac-test binary to Pi ---" -ForegroundColor Yellow
    scp $DacTestBinary "${TargetHost}:/opt/lasertargets/dac-test"
    ssh $TargetHost "chmod +x /opt/lasertargets/dac-test"

    # 5. Copy shared library if present
    if (Test-Path $HeliosLibrary) {
        Write-Host "--- Copying libHeliosLaserDAC.so library ---" -ForegroundColor Yellow
        scp $HeliosLibrary "${TargetHost}:/opt/lasertargets/lib/"
    }

    # 6. Execute dac-test with sudo
    if ($Scenario -eq "all") {
        Write-Host "--- Executing ALL scenarios (info, blink 5s, box 5s, stress 10s, sweep) ---" -ForegroundColor Green
        Write-Host ""
        $allCmd = @(
            "echo '=== 1. INFO ==='",
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test info",
            "echo ''",
            "echo '=== 2. BLINK (5s) ==='",
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test blink 5",
            "echo ''",
            "echo '=== 3. BOX (5s) ==='",
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test box 5",
            "echo ''",
            "echo '=== 4. STRESS (10s) ==='",
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test stress 10",
            "echo ''",
            "echo '=== 5. SWEEP ==='",
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test sweep"
        ) -join " && "
        ssh -t $TargetHost $allCmd
    } else {
        Write-Host "--- Executing: sudo dac-test $Scenario $Duration ---" -ForegroundColor Green
        Write-Host ""
        $testCmd = if ($Duration -gt 0) {
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test $Scenario $Duration"
        } else {
            "sudo LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/dac-test $Scenario"
        }
        ssh -t $TargetHost $testCmd
    }
}
finally {
    # 7. Always restart server service when finished (or on exit/Ctrl+C)
    Write-Host ""
    Write-Host "--- Restoring lasertargets-server service ---" -ForegroundColor Yellow
    ssh $TargetHost "sudo systemctl start lasertargets-server"
    Write-Host "=== DAC Test session ended, server service restarted ===" -ForegroundColor Green
}
