#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Optimize Raspberry Pi OS settings for real-time USB laser projection.

.DESCRIPTION
    Applies OS-level performance tweaks to the Raspberry Pi:
    1. Sets CPU scaling governor to 'performance'.
    2. Configures kernel command line to disable USB autosuspend (/boot/firmware/cmdline.txt).
    3. Creates a systemd service to persist 'performance' CPU governor across reboots.

.PARAMETER TargetHost
    The Raspberry Pi host (e.g. lasertargets.local or lasertargets@192.168.1.246).
#>
param(
    [string]$TargetHost = "lasertargets.local"
)

$ErrorActionPreference = "Stop"

if ($TargetHost -notmatch "@") {
    $TargetHost = "lasertargets@$TargetHost"
}

Write-Host "=== Optimizing Raspberry Pi System Settings ($TargetHost) ===" -ForegroundColor Cyan

# 1. Set CPU Governor to Performance immediately
Write-Host "--- 1. Setting CPU Governor to 'performance' ---" -ForegroundColor Yellow
$null = ssh $TargetHost "echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1 || true"

# 2. Make CPU Performance Governor persistent via tmpfiles.d / systemd service
Write-Host "--- 2. Setting up persistent CPU performance governor ---" -ForegroundColor Yellow
$serviceCmd = @'
sudo bash -c 'cat << "EOF" > /etc/systemd/system/cpu-performance.service
[Unit]
Description=Set CPU Scaling Governor to Performance
After=sys-devices-system-cpu-cpu0-cpufreq-scaling_governor.device

[Service]
Type=oneshot
ExecStart=/bin/sh -c "echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"

[Install]
WantedBy=multi-user.target
EOF
'
sudo systemctl daemon-reload
sudo systemctl enable --now cpu-performance.service
'@
$null = ssh $TargetHost $serviceCmd

# 3. Disable USB Autosuspend in sysfs and cmdline.txt
Write-Host "--- 3. Disabling Linux USB Autosuspend ---" -ForegroundColor Yellow
$null = ssh $TargetHost "echo -1 | sudo tee /sys/module/usbcore/parameters/autosuspend >/dev/null 2>&1 || true"

$cmdlineCmd = @'
CMDLINE=""
if [ -f /boot/firmware/cmdline.txt ]; then
    CMDLINE="/boot/firmware/cmdline.txt"
elif [ -f /boot/cmdline.txt ]; then
    CMDLINE="/boot/cmdline.txt"
fi

if [ -n "$CMDLINE" ]; then
    if ! grep -q "usbcore.autosuspend=-1" "$CMDLINE"; then
        echo "Adding usbcore.autosuspend=-1 to $CMDLINE..."
        sudo sed -i 's/$/ usbcore.autosuspend=-1/' "$CMDLINE"
    else
        echo "usbcore.autosuspend=-1 is already configured in $CMDLINE."
    fi
fi
'@
$null = ssh $TargetHost $cmdlineCmd

Write-Host ""
Write-Host "=== Pi System Optimization Complete ===" -ForegroundColor Green
Write-Host "CPU frequency governor set to performance and USB autosuspend disabled." -ForegroundColor Gray
