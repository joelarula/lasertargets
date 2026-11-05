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
      - camera     : Probe CSI and USB camera hardware specifications.
      - audio      : Probe 3.5mm jack, USB DACs, I2S, and ALSA playback devices.
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
    [ValidateSet("check", "camera", "audio", "check-sd", "optimize", "deploy", "run", "dac-test", "logs")]
    [string]$Action = "check",

    [string]$TargetHost = "lasertargets.local",

    [ValidateSet("info", "debug", "trace", "warn", "error")]
    [string]$LogLevel = "info",

    [ValidateSet("all", "info", "blink", "box", "stress", "sweep", "shapes")]
    [string]$Scenario = "shapes",

    [int]$Duration = 0,

    [string]$BootDrive = "D:",

    [switch]$StartService,

    [switch]$Follow,

    [switch]$Detailed,

    [switch]$TestAudio
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
    Write-Host "=== Raspberry Pi Full Status Cycle ($TargetHost) ===" -ForegroundColor Cyan

    $pureHost = $TargetHost -replace "^.*@", ""

    # --------------------------------------------------------------------------
    # PHASE 1: Connection & Reachability
    # --------------------------------------------------------------------------
    Write-Host "`n[Phase 1/3] Connection & Reachability" -ForegroundColor Yellow
    Write-Host "--------------------------------------" -ForegroundColor DarkGray

    # 1. Ping
    $pingOk = Test-Connection -ComputerName $pureHost -Count 2 -Quiet -ErrorAction SilentlyContinue
    if ($pingOk) {
        Write-Host "  [PASS] Network Ping OK to $pureHost" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] Network Ping timed out for $pureHost" -ForegroundColor Yellow
    }

    # 2. SSH Connection
    $sshCheck = ssh -o BatchMode=yes -o ConnectTimeout=5 $TargetHost "echo OK" 2>&1
    if ($sshCheck -eq "OK") {
        Write-Host "  [PASS] SSH login verified" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] Cannot establish SSH connection to $TargetHost" -ForegroundColor Red
        Write-Host "         Error: $sshCheck" -ForegroundColor DarkGray
        exit 1
    }

    # --------------------------------------------------------------------------
    # PHASE 2: System Status & Health Parameters
    # --------------------------------------------------------------------------
    Write-Host "`n[Phase 2/3] System Status & Health Parameters" -ForegroundColor Yellow
    Write-Host "----------------------------------------------" -ForegroundColor DarkGray

    # 3. System info
    $sysInfo = ssh $TargetHost "uname -srm; uptime -p" 2>$null
    foreach ($line in $sysInfo) {
        Write-Host "  [INFO] $line" -ForegroundColor Gray
    }

    # 4. Temperature & Throttling
    $healthInfo = ssh $TargetHost "vcgencmd measure_temp 2>/dev/null || cat /sys/class/thermal/thermal_zone0/temp 2>/dev/null | awk '{printf \"temp=%.1f''C\n\", `$1/1000}'; vcgencmd get_throttled 2>/dev/null || true" 2>$null
    if ($healthInfo) {
        if ($healthInfo.Length -ge 1) {
            Write-Host "  [INFO] CPU Temperature: $($healthInfo[0])" -ForegroundColor Gray
        }
        if ($healthInfo.Length -ge 2) {
            $thr = $healthInfo[1]
            if ($thr -match "0x0") {
                Write-Host "  [PASS] Power & Thermal Throttle: $thr (Normal, no undervoltage)" -ForegroundColor Green
            } else {
                Write-Host "  [WARN] Power & Thermal Throttle: $thr (Check power supply / cooling!)" -ForegroundColor Yellow
            }
        }
    }

    # 5. Memory & Storage
    $resInfo = ssh $TargetHost "free -h | awk '/^Mem:/ {print \"RAM: \" `$3 \" used / \" `$2 \" total\"}'; df -h / | awk 'NR==2 {print \"Disk (/): \" `$3 \" used / \" `$2 \" total (\" `$5 \" used)\"}'" 2>$null
    foreach ($line in $resInfo) {
        Write-Host "  [INFO] $line" -ForegroundColor Gray
    }

    # 6. CPU Governor
    $gov = ssh $TargetHost "cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unknown" 2>$null
    if ($gov -eq "performance") {
        Write-Host "  [PASS] CPU Governor: '$gov' (Optimized for real-time laser galvo timing)" -ForegroundColor Green
    } else {
        Write-Host "  [WARN] CPU Governor: '$gov' (Run '.\scripts\pi.ps1 optimize' to set to 'performance')" -ForegroundColor Yellow
    }

    # --------------------------------------------------------------------------
    # PHASE 3: Peripherals, Devices & Services
    # --------------------------------------------------------------------------
    Write-Host "`n[Phase 3/3] Peripherals, Devices & Services" -ForegroundColor Yellow
    Write-Host "--------------------------------------------" -ForegroundColor DarkGray

    # 7. USB DAC hardware
    $lsusb = ssh $TargetHost "lsusb 2>/dev/null" 2>$null
    $dacFound = $false
    foreach ($l in $lsusb) {
        if ($l -match "(?i)helios|laser|ftdi|1105|2408") {
            $dacFound = $true
            Write-Host "  [PASS] Laser DAC USB hardware detected: $l" -ForegroundColor Green
        }
    }
    if (-not $dacFound) {
        Write-Host "  [WARN] No recognized Laser DAC detected in lsusb" -ForegroundColor Yellow
    }

    # 8. Camera hardware
    $camInfo = ssh $TargetHost "rpicam-hello --list-cameras 2>/dev/null || libcamera-hello --list-cameras 2>/dev/null || v4l2-ctl --list-devices 2>/dev/null || echo 'none'" 2>$null
    if ($camInfo -and $camInfo -notmatch "none|No cameras available") {
        Write-Host "  [PASS] Camera hardware detected:" -ForegroundColor Green
        foreach ($line in $camInfo) {
            Write-Host "    $line" -ForegroundColor Gray
        }
    } else {
        Write-Host "  [WARN] No active camera detected (run '.\scripts\pi.ps1 camera' for deep probe)" -ForegroundColor Yellow
    }

    # 9. Audio hardware & 3.5mm jack
    $audioCards = ssh $TargetHost "aplay -l 2>/dev/null | grep -E '^card' || echo 'none'" 2>$null
    if ($audioCards -and $audioCards -ne "none") {
        Write-Host "  [PASS] Audio playback devices detected (run '.\scripts\pi.ps1 audio' for details):" -ForegroundColor Green
        foreach ($line in $audioCards) {
            Write-Host "    $line" -ForegroundColor Gray
        }
    } else {
        Write-Host "  [WARN] No active audio playback cards in ALSA (run '.\scripts\pi.ps1 audio' to inspect)" -ForegroundColor Yellow
    }

    # 10. Service status
    $svcStatus = ssh $TargetHost "systemctl is-active lasertargets-server 2>/dev/null || echo inactive" 2>$null
    Write-Host "  [INFO] lasertargets-server.service: $svcStatus" -ForegroundColor Gray

    Write-Host "`nStatus cycle complete." -ForegroundColor Cyan
    exit 0
}

# ==============================================================================
# ACTION: camera
# ==============================================================================
if ($Action -eq "camera") {
    Write-Host "=== Probing Camera Hardware on Raspberry Pi ($TargetHost) ===" -ForegroundColor Cyan

    Write-Host "`n--- 1. CSI / libcamera Detection ---" -ForegroundColor Yellow
    $csi = ssh $TargetHost "rpicam-hello --list-cameras 2>&1 || libcamera-hello --list-cameras 2>&1" 2>$null
    if ($csi -and $csi -notmatch "No cameras available") {
        Write-Host "  [PASS] CSI Camera(s) found:" -ForegroundColor Green
        foreach ($line in $csi) { Write-Host "    $line" -ForegroundColor Gray }
    } else {
        Write-Host "  [INFO] No active CSI camera detected via libcamera." -ForegroundColor Yellow
    }

    Write-Host "`n--- 2. V4L2 Video Devices ---" -ForegroundColor Yellow
    $v4l2 = ssh $TargetHost "v4l2-ctl --list-devices 2>/dev/null || echo 'v4l2-ctl not installed'" 2>$null
    if ($v4l2 -and $v4l2 -ne "v4l2-ctl not installed") {
        foreach ($line in $v4l2) { Write-Host "    $line" -ForegroundColor Gray }
    } else {
        Write-Host "  [WARN] v4l2-ctl not available on Pi (sudo apt install v4l-utils)." -ForegroundColor Yellow
    }

    Write-Host "`n--- 3. Kernel Driver & I2C Probing (dmesg) ---" -ForegroundColor Yellow
    $dmesg = ssh $TargetHost "dmesg | grep -iE 'camera|imx|ov5647|unicam|csi|sensor' | tail -n 15 || true" 2>$null
    if ($dmesg) {
        foreach ($line in $dmesg) { Write-Host "    $line" -ForegroundColor Gray }
    } else {
        Write-Host "  [INFO] No camera kernel messages in dmesg." -ForegroundColor Gray
    }

    Write-Host "`n--- 4. Formats & Resolutions for /dev/video* ---" -ForegroundColor Yellow
    $detailCmd = 'for dev in $(ls /dev/video* 2>/dev/null); do echo "=== Device: $dev ==="; v4l2-ctl -d "$dev" --list-formats-ext 2>/dev/null || true; done'
    $formats = ssh $TargetHost $detailCmd 2>$null
    if ($formats) {
        foreach ($line in $formats) { Write-Host "    $line" -ForegroundColor Gray }
    }

    Write-Host "`nCamera probe complete." -ForegroundColor Cyan
    exit 0
}

# ==============================================================================
# ACTION: audio
# ==============================================================================
if ($Action -eq "audio") {
    Write-Host "=== Probing Audio Hardware on Raspberry Pi ($TargetHost) ===" -ForegroundColor Cyan

    Write-Host "`n--- 1. ALSA Playback Cards (aplay -l) ---" -ForegroundColor Yellow
    $cards = ssh $TargetHost "aplay -l 2>&1" 2>$null
    if ($cards -and $cards -notmatch "no soundcards found") {
        Write-Host "  [PASS] Playback soundcard(s) detected:" -ForegroundColor Green
        foreach ($line in $cards) { Write-Host "    $line" -ForegroundColor Gray }
    } else {
        Write-Host "  [WARN] No soundcards detected via aplay." -ForegroundColor Yellow
    }

    Write-Host "`n--- 2. Built-in 3.5mm Headphone Jack Status ---" -ForegroundColor Yellow
    $hpCmd = "echo '--- Kernel Module (snd_bcm2835) ---'; lsmod | grep -E 'snd_bcm2835|bcm2835_audio' || echo 'Module snd_bcm2835 NOT loaded (check dtparam=audio=on)'; echo '--- Firmware config (dtparam=audio) ---'; grep -E '^\s*dtparam=audio' /boot/firmware/config.txt /boot/config.txt 2>/dev/null || echo 'dtparam=audio not found'; echo '--- ALSA Card Identification ---'; aplay -l 2>/dev/null | grep -iE 'headphone|bcm2835' || echo 'No bcm2835 headphone device listed in ALSA'"
    $hpCheck = ssh $TargetHost $hpCmd 2>$null
    if ($hpCheck) {
        foreach ($line in $hpCheck) { Write-Host "  $line" -ForegroundColor Gray }
    }

    Write-Host "`n--- 3. USB Audio Hardware (DACs / Soundcards) ---" -ForegroundColor Yellow
    $usbAudio = ssh $TargetHost "lsusb 2>/dev/null | grep -iE 'audio|sound|c-media|pcm|dac|dac-|headset|speaker' || echo 'No dedicated USB audio devices in lsusb'" 2>$null
    if ($usbAudio) {
        foreach ($line in $usbAudio) { Write-Host "  $line" -ForegroundColor Gray }
    }

    Write-Host "`n--- 4. I2S Audio Overlays ---" -ForegroundColor Yellow
    $i2s = ssh $TargetHost "grep -iE 'dtoverlay=.*(hifiberry|i2s|max98357|audio|dac|pcm5102)' /boot/firmware/config.txt /boot/config.txt 2>/dev/null || echo 'No active I2S audio overlays configured in config.txt'" 2>$null
    if ($i2s) {
        foreach ($line in $i2s) { Write-Host "  $line" -ForegroundColor Gray }
    }

    Write-Host "`n--- 5. User Permissions ('audio' group) ---" -ForegroundColor Yellow
    $groups = ssh $TargetHost "id -nG 2>/dev/null" 2>$null
    if ($groups -match "\baudio\b") {
        Write-Host "  [PASS] Target user is in the 'audio' group." -ForegroundColor Green
    } else {
        Write-Host "  [WARN] Target user is NOT in the 'audio' group (run: sudo usermod -a -G audio <user>)" -ForegroundColor Yellow
    }

    if ($Detailed) {
        Write-Host "`n--- 6. Detailed ALSA PCMs & Mixer Controls ---" -ForegroundColor Yellow
        $detailCmd = "echo '=== ALSA PCM Definitions ==='; aplay -L 2>/dev/null | head -n 35; echo ''; echo '=== Mixer Controls ==='; amixer scontrols 2>/dev/null; echo ''; echo '=== Headphone / Master Volume ==='; amixer sget Headphones 2>/dev/null || amixer sget Master 2>/dev/null || amixer 2>/dev/null | head -n 30"
        $detail = ssh $TargetHost $detailCmd 2>$null
        if ($detail) {
            foreach ($line in $detail) { Write-Host "  $line" -ForegroundColor Gray }
        }
    } else {
        Write-Host "`n[TIP] Pass '-Detailed' to view all ALSA PCM devices and mixer volume levels." -ForegroundColor DarkGray
    }

    if ($TestAudio) {
        Write-Host "`n--- 7. Playing Audio Test Tone (1s Sine Wave) ---" -ForegroundColor Yellow
        $tone = ssh $TargetHost "speaker-test -D plughw:CARD=Headphones,DEV=0 -t sine -f 440 -c 2 -l 1 2>&1 || speaker-test -D default -t sine -f 440 -c 2 -l 1 2>&1 || echo 'speaker-test failed or not installed'" 2>$null
        if ($tone) {
            foreach ($line in $tone) { Write-Host "  $line" -ForegroundColor Gray }
        }
    } else {
        Write-Host "`n[TIP] Pass '-TestAudio' to play a 440Hz test tone through the 3.5mm jack or default audio output." -ForegroundColor DarkGray
    }

    Write-Host "`nAudio probe complete." -ForegroundColor Cyan
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

