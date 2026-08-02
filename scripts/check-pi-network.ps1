# check-pi-network.ps1
#
# Checks that the Pi is reachable over the network and SSH is working.
# Run this AFTER the Pi has booted with the SD card inserted.
#
# Usage:
#   .\scripts\check-pi-network.ps1 -PiHost 192.168.1.50
#   .\scripts\check-pi-network.ps1 -PiHost lasertargets.local
#   .\scripts\check-pi-network.ps1 -PiHost 192.168.1.50 -PiUser myuser

param(
    [string]$PiHost     = "lasertargets.local",
    [string]$PiUser     = "lasertargets",
    [int]   $SshTimeout = 8
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
function Section($t){ Write-Host ""; Write-Host "--- $t ---" -ForegroundColor Cyan }

$sshUser = "${PiUser}@${PiHost}"

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  LaserTargets - Pi Network Check"     -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Pi host : $sshUser"
Write-Host "======================================" -ForegroundColor Cyan

# ===========================================================================
# 1. Ping
# ===========================================================================
Section "1. Reachability"

$pingOk = Test-Connection -ComputerName $PiHost -Count 2 -Quiet -ErrorAction SilentlyContinue
if ($pingOk) {
    try {
        $ip = ([System.Net.Dns]::GetHostAddresses($PiHost) |
               Where-Object { $_.AddressFamily -eq "InterNetwork" } |
               Select-Object -First 1).IPAddressToString
        Pass "Ping OK ($PiHost -> $ip)"
    } catch {
        Pass "Ping OK ($PiHost)"
    }
} else {
    Fail "Cannot ping $PiHost"
    Write-Host ""
    Write-Host "  Possible causes:" -ForegroundColor Yellow
    Write-Host "    - Pi not powered on or still booting (wait ~60 s after power-on)"
    Write-Host "    - mDNS not resolving 'lasertargets.local' - try the raw IP instead:"
    Write-Host "        .\scripts\check-pi-network.ps1 -PiHost 192.168.x.x"
    Write-Host "    - Pi not on the same network / Wi-Fi not configured"
    Write-Host "    - Wrong hostname (Pi was renamed)"
    Write-Host ""
}

# ===========================================================================
# 2. SSH port
# ===========================================================================
Section "2. SSH Port (TCP 22)"

$tcpTest = Test-NetConnection -ComputerName $PiHost -Port 22 `
               -WarningAction SilentlyContinue -ErrorAction SilentlyContinue
if ($tcpTest.TcpTestSucceeded) {
    Pass "Port 22 is open on $PiHost"
} else {
    Fail "Port 22 is not reachable on $PiHost"
    Write-Host ""
    Write-Host "  SSH is probably not enabled on the Pi." -ForegroundColor Yellow
    Write-Host "  If the SD card is still available, run: .\scripts\check-sd-card.ps1"
    Write-Host "  Otherwise enable SSH via raspi-config on the Pi itself."
    Write-Host ""
}

# ===========================================================================
# 3. SSH key authentication
# ===========================================================================
Section "3. SSH Key Auth"

$sshOut = & ssh -o BatchMode=yes `
               -o ConnectTimeout=$SshTimeout `
               -o StrictHostKeyChecking=accept-new `
               $sshUser "echo OK" 2>&1
$sshExitCode = $LASTEXITCODE
$sshStr = $sshOut -join " "

if ($sshExitCode -eq 0 -and $sshStr -match "OK") {
    Pass "Key-based login works ($sshUser)"
} elseif ($sshStr -match "Permission denied") {
    Fail "Key authentication failed - public key not on Pi"
    Write-Host ""
    Write-Host "  Fix (Git Bash / WSL / macOS):" -ForegroundColor Yellow
    Write-Host "    ssh-copy-id $sshUser"
    Write-Host ""
    Write-Host "  Fix (Windows PowerShell):" -ForegroundColor Yellow
    Write-Host "    type `$env:USERPROFILE\.ssh\id_ed25519.pub | ssh $sshUser `"mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys`""
    Write-Host ""
    Write-Host "  If you have no key yet, generate one first:" -ForegroundColor Yellow
    Write-Host "    ssh-keygen -t ed25519 -C lasertargets-deploy"
    Write-Host ""
} elseif ($sshStr -match "Connection refused") {
    Fail "Connection refused - SSH daemon is not running on the Pi"
} elseif ($sshStr -match "No route|timed out|Could not resolve") {
    Fail "Could not connect: $sshStr"
    Warn "Run ping check first - the Pi may not be reachable yet"
} else {
    Warn "SSH result inconclusive: $sshStr"
}

# Only continue with Pi-side checks if SSH succeeded
if ($sshExitCode -ne 0) {
    Write-Host ""
    Write-Host "  Skipping Pi-side checks (SSH not working)." -ForegroundColor Yellow
} else {

    # ===========================================================================
    # 4. Pi runtime dependencies
    # ===========================================================================
    Section "4. Pi Runtime Dependencies"

    # /opt/lasertargets
    $hasDeps = (& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                   "test -d /opt/lasertargets && echo YES || echo NO" 2>&1) -join ""
    if ($hasDeps -match "YES") {
        Pass "/opt/lasertargets exists - install-pi-deps.sh has been run"
    } else {
        Warn "/opt/lasertargets not found - one-time setup not done"
        Write-Host ""
        Write-Host "  From the project root, run:" -ForegroundColor Yellow
        Write-Host "    scp deploy/install-pi-deps.sh ${sshUser}:/tmp/" -ForegroundColor White
        Write-Host "    ssh $sshUser sudo bash /tmp/install-pi-deps.sh" -ForegroundColor White
        Write-Host ""
    }

    # libusb-1.0-0
    $libusb = (& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                  "dpkg -l libusb-1.0-0 2>/dev/null | grep -c '^ii' || echo 0" 2>&1) -join ""
    if ($libusb -match "^1") {
        Pass "libusb-1.0-0 installed"
    } else {
        Warn "libusb-1.0-0 not found - run install-pi-deps.sh on the Pi"
    }

    # plugdev group
    $plugdev = (& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                   "groups | grep -c plugdev || echo 0" 2>&1) -join ""
    if ($plugdev -match "^1") {
        Pass "User '$PiUser' is in the plugdev group"
    } else {
        Warn "User '$PiUser' is NOT in the plugdev group"
        Write-Host "    Fix: ssh $sshUser 'sudo usermod -aG plugdev $PiUser'" -ForegroundColor Yellow
        Write-Host "    Then reboot the Pi for the change to take effect." -ForegroundColor Yellow
    }

    # ===========================================================================
    # 5. Deployed binary
    # ===========================================================================
    Section "5. Deployed Server Binary"

    $hasBin = (& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                  "test -x /opt/lasertargets/server && echo YES || echo NO" 2>&1) -join ""
    if ($hasBin -match "YES") {
        $arch = (& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                     "file /opt/lasertargets/server 2>/dev/null" 2>&1) -join ""
        Pass "Binary present: $arch"
    } else {
        Warn "No binary at /opt/lasertargets/server - not yet deployed"
        Write-Host "    Run: .\scripts\build-pi.sh  then  .\scripts\deploy-pi.sh $sshUser" -ForegroundColor Yellow
    }

    $hasLib = (& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                  "test -f /opt/lasertargets/lib/libHeliosLaserDAC.so && echo YES || echo NO" 2>&1) -join ""
    if ($hasLib -match "YES") {
        Pass "libHeliosLaserDAC.so present in /opt/lasertargets/lib/"
    } else {
        Warn "libHeliosLaserDAC.so not deployed yet (expected after deploy-pi.sh)"
    }

    # ===========================================================================
    # 6. Service status
    # ===========================================================================
    Section "6. systemd Service"

    $svcActive = ((& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                      "systemctl is-active lasertargets-server 2>/dev/null || echo inactive" 2>&1) -join "").Trim()
    $svcEnabled = ((& ssh -o BatchMode=yes -o ConnectTimeout=$SshTimeout $sshUser `
                       "systemctl is-enabled lasertargets-server 2>/dev/null || echo disabled" 2>&1) -join "").Trim()

    if ($svcActive -eq "active") {
        Pass "lasertargets-server is running"
    } elseif ($svcActive -eq "failed") {
        Fail "lasertargets-server is in FAILED state"
        Write-Host "    Check logs: ssh $sshUser 'sudo journalctl -u lasertargets-server -n 30 --no-pager'" -ForegroundColor Yellow
    } else {
        Warn "lasertargets-server is 'inactive' (not deployed or started yet)"
        Write-Host "    Start: ssh $sshUser 'sudo systemctl start lasertargets-server'" -ForegroundColor Yellow
    }

    if ($svcEnabled -match "^enabled") {
        Pass "lasertargets-server is enabled (auto-starts on boot)"
    } else {
        Warn "lasertargets-server is not enabled for auto-start"
        Write-Host "    Enable: ssh $sshUser 'sudo systemctl enable lasertargets-server'" -ForegroundColor Yellow
    }
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
    Write-Host "  Pi is ready." -ForegroundColor Green
    Write-Host ""
    Write-Host "  Build and deploy:" -ForegroundColor Cyan
    Write-Host "    .\scripts\build-pi.sh"
    Write-Host "    .\scripts\deploy-pi.sh $sshUser"
    Write-Host ""
    Write-Host "  Follow live output:" -ForegroundColor Cyan
    Write-Host "    ssh $sshUser 'sudo journalctl -u lasertargets-server -f'"
}
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""
