#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Unified management script for LaserTargets on the Raspberry Pi.

.DESCRIPTION
    Consolidates diagnostics, provisioning, system optimization, deployment,
    interactive execution, and log viewing for the Raspberry Pi.

.PARAMETER Action
    The action to execute:
      - check      : Ping, SSH, CPU governor, and USB DAC detection.
      - check-sd   : Validate inserted SD card bootfs partition before first boot.
      - optimize   : Apply CPU performance governor & disable USB autosuspend.
      - deploy     : Copy binaries, libraries, assets, and systemd units to the Pi.
      - run        : Run server (or dac-test) interactively with direct terminal output.
      - dac-test   : Run DAC hardware test scenario interactively on the Pi.
      - logs       : View or follow systemd journal logs.

.PARAMETER TargetHost
    The Raspberry Pi SSH host (e.g., lasertargets.local or 192.168.1.50). Defaults to lasertargets.local.

.PARAMETER LogLevel
    Log verbosity for interactive run (info, debug, trace, warn, error). Defaults to 'info'.

.PARAMETER Scenario
    DAC test scenario for 'dac-test' action (all, info, blink, box, stress, sweep, shapes). Defaults to 'shapes'.

.PARAMETER Duration
    Duration in seconds for DAC test scenario (0 = indefinite / until Ctrl+C). Defaults to 0.

.PARAMETER BootDrive
    Drive letter for 'check-sd' action (e.g. D:). Defaults to 'D:'.

.PARAMETER StartService
    When deploying, enable and start the systemd background service. Defaults to false (leaves service stopped so USB DAC is free for interactive testing).

.PARAMETER Follow
    Follow live log output for 'logs' action.

.EXAMPLE
    .\scripts\pi.ps1 check
    .\scripts\pi.ps1 optimize
    .\scripts\pi.ps1 deploy
    .\scripts\pi.ps1 run -LogLevel debug
    .\scripts\pi.ps1 dac-test -Scenario box -Duration 30
    .\scripts\pi.ps1 logs -Follow
#>
param(
    [Parameter(Position = 0)]
    [ValidateSet("check", "check-sd", "optimize", "deploy", "run", "dac-test", "logs")]
    [string]$Action = "check",

    [string]$TargetHost = "lasertargets.local",

    [ValidateSet("info", "debug", "trace", "warn", "error")]
    [string]$LogLevel = "info",

    [ValidateSet("all", "info", "blink", "box", "stress", "sweep", "shapes")]
    [string]$Scenario = "shapes",

    [int]$Duration = 0,

    [string]$BootDrive = "D:",

    [switch]$StartService,

    [switch]$Follow
)

$ErrorActionPreference = "Stop"

if ($TargetHost -notmatch "@") {
    $TargetHost = "lasertargets@$TargetHost"
}

$ScriptDir   = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path (Join-Path $ScriptDir "..")
$DistDir     = Join-Path $ProjectRoot "dist\pi"

# ==============================================================================
# ACTION: check
# ==============================================================================
if ($Action -eq "check") {
    Write-Host "=== Checking Raspberry Pi Status ($TargetHost) ===" -ForegroundColor Cyan

    $pureHost = $TargetHost -replace "^.*@", ""

    # 1. Ping
    Write-Host "`n--- 1. Network Ping ---" -ForegroundColor Yellow
    $pingOk = Test-Connection -ComputerName $pureHost -Count 2 -Quiet -ErrorAction SilentlyContinue
    if ($pingOk) {
        Write-Host "  [PASS] Ping OK to $pureHost" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] Ping failed or timed out for $pureHost" -ForegroundColor Yellow
    }

    # 2. SSH Connection
    Write-Host "`n--- 2. SSH Connectivity ---" -ForegroundColor Yellow
    $sshCheck = ssh -o BatchMode=yes -o ConnectTimeout=5 $TargetHost "echo OK" 2>&1
    if ($sshCheck -eq "OK") {
        Write-Host "  [PASS] SSH login successful" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Cannot establish SSH connection to $TargetHost" -ForegroundColor Red
        Write-Host "         Error: $sshCheck" -ForegroundColor DarkGray
        exit 1
    }

    # 3. System info
    Write-Host "`n--- 3. System Information ---" -ForegroundColor Yellow
    $sysInfo = ssh $TargetHost "uname -srm; uptime -p" 2>$null
    foreach ($line in $sysInfo) {
        Write-Host "  [INFO] $line" -ForegroundColor Gray
    }

    # 4. CPU Governor
    Write-Host "`n--- 4. CPU Governor ---" -ForegroundColor Yellow
    $gov = ssh $TargetHost "cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unknown" 2>$null
    if ($gov -eq "performance") {
        Write-Host "  [PASS] CPU Governor is '$gov'" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] CPU Governor is '$gov' (run '.\scripts\pi.ps1 optimize' to set to 'performance')" -ForegroundColor Yellow
    }

    # 5. USB DAC hardware
    Write-Host "`n--- 5. USB DAC Hardware ---" -ForegroundColor Yellow
    $lsusb = ssh $TargetHost "lsusb 2>/dev/null" 2>$null
    $dacFound = $false
    foreach ($l in $lsusb) {
        if ($l -match "(?i)helios|laser|ftdi|1105|2408") {
            $dacFound = $true
            Write-Host "  [PASS] Detected USB Device: $l" -ForegroundColor Green
        }
    }
    if (-not $dacFound) {
        Write-Host "  [WARN] No recognized Laser DAC detected in lsusb" -ForegroundColor Yellow
    }

    # 6. Service status
    Write-Host "`n--- 6. Service Status ---" -ForegroundColor Yellow
    $svcStatus = ssh $TargetHost "systemctl is-active lasertargets-server 2>/dev/null || echo inactive" 2>$null
    Write-Host "  [INFO] lasertargets-server.service: $svcStatus" -ForegroundColor Gray

    Write-Host "`nCheck complete." -ForegroundColor Cyan
    exit 0
}

# ==============================================================================
# ACTION: check-sd
# ==============================================================================
if ($Action -eq "check-sd") {
    Write-Host "=== Checking SD Card Boot Partition ($BootDrive) ===" -ForegroundColor Cyan
    if (-not (Test-Path "$BootDrive\")) {
        Write-Error "Drive $BootDrive not found. Make sure the SD card is inserted and accessible."
    }

    $hasConfig  = (Test-Path "$BootDrive\config.txt") -or (Test-Path "$BootDrive\config.txt.bak")
    $hasCmdline = (Test-Path "$BootDrive\cmdline.txt") -or (Test-Path "$BootDrive\cmdline.txt.bak")

    if ($hasConfig -or $hasCmdline) {
        Write-Host "  [PASS] Raspberry Pi bootfs detected on $BootDrive" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] config.txt/cmdline.txt not found on $BootDrive" -ForegroundColor Red
        exit 1
    }

    if (Test-Path "$BootDrive\cmdline.txt") {
        $cmdline = Get-Content "$BootDrive\cmdline.txt" -Raw
        if ($cmdline -match "usbcore\.autosuspend=-1") {
            Write-Host "  [PASS] USB autosuspend is disabled in cmdline.txt" -ForegroundColor Green
        } else {
            Write-Host "  [WARN] usbcore.autosuspend=-1 not in cmdline.txt (will be configured by 'optimize')" -ForegroundColor Yellow
        }
    }
    exit 0
}

# ==============================================================================
# ACTION: optimize
# ==============================================================================
if ($Action -eq "optimize") {
    Write-Host "=== Optimizing Raspberry Pi System Settings ($TargetHost) ===" -ForegroundColor Cyan

    Write-Host "`n[1/3] Setting CPU Governor to 'performance'..." -ForegroundColor Yellow
    $null = ssh $TargetHost "echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1 || true"

    Write-Host "[2/3] Setting up persistent CPU performance systemd service..." -ForegroundColor Yellow
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

    Write-Host "[3/3] Disabling Linux USB Autosuspend..." -ForegroundColor Yellow
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
        sudo sed -i 's/$/ usbcore.autosuspend=-1/' "$CMDLINE"
        echo "  [OK] Added usbcore.autosuspend=-1 to $CMDLINE (active after reboot)"
    else
        echo "  [OK] usbcore.autosuspend=-1 already present in $CMDLINE"
    fi
fi
'@
    $null = ssh $TargetHost $cmdlineCmd

    Write-Host "`nOptimization complete." -ForegroundColor Green
    exit 0
}

# ==============================================================================
# ACTION: deploy
# ==============================================================================
if ($Action -eq "deploy") {
    $ServerBinary  = Join-Path $DistDir "server"
    $HeliosLibrary = Join-Path $DistDir "libHeliosLaserDAC.so"
    $DacTestBinary = Join-Path $DistDir "dac-test"

    Write-Host "=== Deploying LaserTargets to $TargetHost ===" -ForegroundColor Cyan

    if (-not (Test-Path $ServerBinary)) {
        Write-Error "Build artifact not found: $ServerBinary. Run .\scripts\build-arm64.ps1 first."
    }

    Write-Host "`n--- 1. Stopping existing service ---" -ForegroundColor Yellow
    $null = ssh $TargetHost "sudo systemctl stop lasertargets-server 2>/dev/null || true"

    Write-Host "--- 2. Ensuring directory structure ---" -ForegroundColor Yellow
    $null = ssh $TargetHost "sudo mkdir -p /opt/lasertargets/lib /opt/lasertargets/stats && sudo chown -R lasertargets:lasertargets /opt/lasertargets"

    $ServerSizeMB = [math]::Round((Get-Item $ServerBinary).Length / 1MB, 1)
    Write-Host "--- 3. Deploying server binary ($ServerSizeMB MB) ---" -ForegroundColor Yellow
    $null = ssh $TargetHost "rm -f /opt/lasertargets/server"
    scp $ServerBinary "${TargetHost}:/opt/lasertargets/server"
    $null = ssh $TargetHost "chmod +x /opt/lasertargets/server && sudo setcap cap_sys_nice+eip /opt/lasertargets/server 2>/dev/null || true"

    if (Test-Path $DacTestBinary) {
        Write-Host "--- 4. Deploying dac-test binary ---" -ForegroundColor Yellow
        $null = ssh $TargetHost "rm -f /opt/lasertargets/dac-test"
        scp $DacTestBinary "${TargetHost}:/opt/lasertargets/dac-test"
        $null = ssh $TargetHost "chmod +x /opt/lasertargets/dac-test"
    }

    if (Test-Path $HeliosLibrary) {
        Write-Host "--- 5. Deploying Helios DAC library ---" -ForegroundColor Yellow
        scp $HeliosLibrary "${TargetHost}:/opt/lasertargets/lib/"
        scp $HeliosLibrary "${TargetHost}:/opt/lasertargets/"
        $null = ssh $TargetHost "sudo cp /opt/lasertargets/libHeliosLaserDAC.so /usr/lib/libHeliosLaserDAC.so 2>/dev/null || true"
    }

    $AssetsDir = Join-Path $ProjectRoot "assets"
    if (Test-Path $AssetsDir) {
        Write-Host "--- 6. Deploying assets folder ---" -ForegroundColor Yellow
        scp -r $AssetsDir "${TargetHost}:/opt/lasertargets/"
    }

    $ServiceFile = Join-Path $ProjectRoot "deploy\lasertargets-server.service"
    if (Test-Path $ServiceFile) {
        Write-Host "--- 7. Installing systemd service unit ---" -ForegroundColor Yellow
        scp $ServiceFile "${TargetHost}:/tmp/lasertargets-server.service"
        $null = ssh $TargetHost "sudo mv /tmp/lasertargets-server.service /etc/systemd/system/ && sudo systemctl daemon-reload"
    }

    $LogrotateConfig = Join-Path $ProjectRoot "deploy\lasertargets-logrotate"
    if (Test-Path $LogrotateConfig) {
        Write-Host "--- 8. Installing logrotate configuration ---" -ForegroundColor Yellow
        scp $LogrotateConfig "${TargetHost}:/tmp/lasertargets-logrotate"
        $null = ssh $TargetHost "sudo mv /tmp/lasertargets-logrotate /etc/logrotate.d/lasertargets && sudo chown root:root /etc/logrotate.d/lasertargets && sudo chmod 644 /etc/logrotate.d/lasertargets"
    }

    if ($StartService) {
        Write-Host "--- 9. Enabling & starting background service ---" -ForegroundColor Yellow
        $null = ssh $TargetHost "sudo systemctl enable --now lasertargets-server"
        ssh $TargetHost "sudo systemctl status lasertargets-server --no-pager"
    } else {
        Write-Host "--- 9. Leaving background service stopped (frees USB DAC for interactive run) ---" -ForegroundColor Yellow
        $null = ssh $TargetHost "sudo systemctl disable --now lasertargets-server 2>/dev/null; sudo killall -9 server 2>/dev/null || true"
    }

    Write-Host "`n=== Deployment complete ===" -ForegroundColor Green
    Write-Host "Run interactively with logs: .\scripts\pi.ps1 run" -ForegroundColor Cyan
    Write-Host "Or start background service:  ssh $TargetHost 'sudo systemctl start lasertargets-server'" -ForegroundColor Gray
    exit 0
}

# ==============================================================================
# ACTION: run
# ==============================================================================
if ($Action -eq "run") {
    Write-Host "=== Running LaserTargets Server Interactively ($TargetHost) ===" -ForegroundColor Cyan
    try {
        Write-Host "--- Ensuring background service is stopped to free USB DAC ---" -ForegroundColor Yellow
        ssh $TargetHost "sudo systemctl stop lasertargets-server 2>/dev/null; sudo killall -9 server 2>/dev/null || true"
        Start-Sleep -Seconds 1

        Write-Host "--- Launching server directly (Press Ctrl+C to stop) ---" -ForegroundColor Green
        Write-Host "Log Level: RUST_LOG=$LogLevel" -ForegroundColor DarkGray
        Write-Host "--------------------------------------------------------" -ForegroundColor Gray

        ssh -t $TargetHost "cd /opt/lasertargets && sudo env LD_LIBRARY_PATH=/opt/lasertargets RUST_LOG=$LogLevel /opt/lasertargets/server"
    } finally {
        Write-Host "`n--- Interactive session closed. Service left stopped. ---" -ForegroundColor Gray
    }
    exit 0
}

# ==============================================================================
# ACTION: dac-test
# ==============================================================================
if ($Action -eq "dac-test") {
    Write-Host "=== Running LaserTargets DAC Hardware Test ($TargetHost) ===" -ForegroundColor Cyan
    $DacTestBinary = Join-Path $DistDir "dac-test"
    $HeliosLibrary = Join-Path $DistDir "libHeliosLaserDAC.so"

    if (-not (Test-Path $DacTestBinary)) {
        Write-Error "dac-test binary not found: $DacTestBinary. Run .\scripts\build-arm64.ps1 first."
    }

    Write-Host "--- Stopping background service & deploying test binary ---" -ForegroundColor Yellow
    $null = ssh $TargetHost "sudo systemctl stop lasertargets-server 2>/dev/null || true"
    scp $DacTestBinary "${TargetHost}:/opt/lasertargets/dac-test"
    $null = ssh $TargetHost "chmod +x /opt/lasertargets/dac-test"

    if (Test-Path $HeliosLibrary) {
        scp $HeliosLibrary "${TargetHost}:/opt/lasertargets/"
        $null = ssh $TargetHost "sudo cp /opt/lasertargets/libHeliosLaserDAC.so /usr/lib/libHeliosLaserDAC.so 2>/dev/null || true"
    }

    $cmd = "cd /opt/lasertargets && sudo env LD_LIBRARY_PATH=/opt/lasertargets /opt/lasertargets/dac-test --scenario $Scenario"
    if ($Duration -gt 0) {
        $cmd += " --duration $Duration"
    }

    Write-Host "--- Executing: $cmd ---" -ForegroundColor Green
    ssh -t $TargetHost $cmd
    exit 0
}

# ==============================================================================
# ACTION: logs
# ==============================================================================
if ($Action -eq "logs") {
    $fFlag = if ($Follow) { "-f" } else { "" }
    ssh $TargetHost "sudo journalctl -u lasertargets-server -n 50 $fFlag --no-pager"
    exit 0
}

