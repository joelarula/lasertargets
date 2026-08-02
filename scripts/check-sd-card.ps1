# check-sd-card.ps1
#
# Checks that D:\ is a valid Raspberry Pi bootfs partition ready for headless setup.
# Run this BEFORE first boot, while the SD card is still in your Windows machine.
#
# Usage:
#   .\scripts\check-sd-card.ps1
#   .\scripts\check-sd-card.ps1 -BootDrive E:

param(
    [string]$BootDrive = "D:"
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
$script:passCount = 0
$script:warnCount = 0
$script:failCount = 0

function Pass($msg) { Write-Host "  [PASS] $msg" -ForegroundColor Green;  $script:passCount++ }
function Warn($msg) { Write-Host "  [WARN] $msg" -ForegroundColor Yellow; $script:warnCount++ }
function Fail($msg) { Write-Host "  [FAIL] $msg" -ForegroundColor Red;    $script:failCount++ }

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  LaserTargets - SD Card Check"        -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Boot drive : $BootDrive"
Write-Host "======================================"  -ForegroundColor Cyan
Write-Host ""

# ---------------------------------------------------------------------------
# Checks
# ---------------------------------------------------------------------------

# 1 - Drive is mounted
if (-not (Test-Path "$BootDrive\")) {
    Fail "Drive $BootDrive not found"
    Write-Host ""
    Write-Host "  Insert the SD card (bootfs partition should appear as $BootDrive)" -ForegroundColor Yellow
    Write-Host "  If it is a different letter, run: .\scripts\check-sd-card.ps1 -BootDrive E:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  FAIL : 1  (cannot continue without the drive)" -ForegroundColor Red
    Write-Host ""
    exit 1
}
Pass "Drive $BootDrive is mounted"

# 2 - Looks like Pi bootfs
$hasConfig  = Test-Path "$BootDrive\config.txt"
$hasCmdline = Test-Path "$BootDrive\cmdline.txt"
if ($hasConfig -or $hasCmdline) {
    Pass "Raspberry Pi bootfs detected (config.txt / cmdline.txt present)"
} else {
    Fail "Does not look like a Pi bootfs - expected config.txt or cmdline.txt at $BootDrive\"
    Warn "Make sure $BootDrive is the bootfs (FAT32) partition, not rootfs (ext4)"
}

# 3 - 64-bit OS (required for aarch64 binary)
if (Test-Path "$BootDrive\kernel8.img") {
    Pass "64-bit kernel present (kernel8.img) - compatible with aarch64 build"
} elseif (Test-Path "$BootDrive\kernel7l.img") {
    Fail "32-bit OS detected (kernel7l.img found, kernel8.img missing)"
    Write-Host "  Flash Raspberry Pi OS (64-bit) - the server binary is aarch64 only" -ForegroundColor Yellow
} else {
    Warn "Cannot confirm OS bitness - ensure you flashed Raspberry Pi OS (64-bit)"
}

# 4 - ssh file: check / create
$sshFile    = "$BootDrive\ssh"
$sshTxtFile = "$BootDrive\ssh.txt"

if (Test-Path $sshTxtFile) {
    Fail "'ssh.txt' found instead of 'ssh' - Windows added a hidden .txt extension"
    Write-Host ""
    Write-Host "  Fix it:" -ForegroundColor Yellow
    Write-Host "    Rename-Item '$sshTxtFile' 'ssh'" -ForegroundColor White
    Write-Host ""
    $fix = Read-Host "  Rename it now? [Y/n]"
    if ($fix -eq "" -or $fix -match "^[Yy]") {
        try {
            Rename-Item $sshTxtFile "ssh" -Force
            Pass "Renamed ssh.txt -> ssh"
        } catch {
            Fail "Rename failed: $_"
        }
    }
}

if (Test-Path $sshFile) {
    Pass "SSH enable file present ($sshFile) - SSH will activate on first boot"
} else {
    Warn "SSH enable file missing at $sshFile"
    Write-Host ""
    Write-Host "  Without this file SSH will NOT be enabled on first boot." -ForegroundColor Yellow
    Write-Host ""
    $create = Read-Host "  Create it now? [Y/n]"
    if ($create -eq "" -or $create -match "^[Yy]") {
        try {
            New-Item -Path $sshFile -ItemType File -Force | Out-Null
            Pass "Created $sshFile - SSH will be enabled on first boot"
        } catch {
            Fail "Could not create $sshFile : $_"
        }
    } else {
        Warn "Skipped - create the file before inserting the SD card into the Pi"
    }
}

# 5 - Drive is writable
try {
    $tmp = "$BootDrive\.lt-write-test"
    [System.IO.File]::WriteAllText($tmp, "ok")
    Remove-Item $tmp -Force
    Pass "$BootDrive is writable"
} catch {
    Fail "$BootDrive is read-only - unlock the SD card write-protect switch"
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Summary"                              -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ("  PASS : " + $script:passCount) -ForegroundColor Green
if ($script:warnCount -gt 0) { Write-Host ("  WARN : " + $script:warnCount) -ForegroundColor Yellow }
else                          { Write-Host ("  WARN : " + $script:warnCount) }
if ($script:failCount -gt 0) {
    Write-Host ("  FAIL : " + $script:failCount) -ForegroundColor Red
    Write-Host ""
    Write-Host "  Fix the FAIL items then re-run this script." -ForegroundColor Red
} else {
    Write-Host ("  FAIL : " + $script:failCount)
    Write-Host ""
    Write-Host "  SD card is ready." -ForegroundColor Green
    Write-Host "  Eject $BootDrive, insert into Pi, and power on." -ForegroundColor Green
    Write-Host "  Then run: .\scripts\check-pi-network.ps1 -PiHost <IP>" -ForegroundColor Cyan
}
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""
