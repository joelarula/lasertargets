#!/usr/bin/env pwsh
<#
.SYNOPSIS
    One-time setup: register a scheduled task on the remote build machine
    so Docker Desktop can be started from SSH.

.DESCRIPTION
    Windows SSH sessions run in Session 0 and cannot start WSL2/GUI apps.
    A scheduled task registered to the logged-in user runs in their GUI
    session (Session 1) and CAN start Docker Desktop + its Linux engine.

    Run this ONCE on initial setup. After that, ensure-remote-docker.ps1
    will use 'schtasks /run' to trigger Docker Desktop if it is not running.

.PARAMETER RemoteHost
    SSH target for the remote build machine, e.g. joel@192.168.1.110.

.PARAMETER RemoteUser
    Windows username on the remote machine (default: joel).
    Used to register the scheduled task under the correct user account.

.EXAMPLE
    .\scripts\setup-remote-docker-task.ps1 -RemoteHost joel@192.168.1.110
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$RemoteHost,

    [string]$RemoteUser = "joel"
)

$ErrorActionPreference = "Stop"

Write-Host "Registering Docker Desktop scheduled task on $RemoteHost ..." -ForegroundColor Cyan

$taskScript = @(
    # Delete old task if it exists
    'schtasks /delete /tn "LaserTargets-StartDocker" /f 2>&1 | Out-Null',

    # Register new task that runs as the interactive user in their session
    '$exe = [System.IO.Path]::Combine($env:ProgramFiles, "Docker\Docker\Docker Desktop.exe")',
    '$action = New-ScheduledTaskAction -Execute $exe',
    # RunOnlyIfLoggedOn ensures it runs in the user session, not session 0
    '$settings = New-ScheduledTaskSettingsSet -RunOnlyIfNetworkAvailable:$false',
    '$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited',
    'Register-ScheduledTask -TaskName "LaserTargets-StartDocker" -Action $action -Principal $principal -Settings $settings -Force | Out-Null',
    'Write-Host "Task registered as: LaserTargets-StartDocker"',
    'Write-Host "Run it anytime with: schtasks /run /tn LaserTargets-StartDocker"',
    # Also enable Docker Desktop auto-start on login (writes registry key)
    '$regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"',
    '$exePath = [System.IO.Path]::Combine($env:ProgramFiles, "Docker\Docker\Docker Desktop.exe")',
    'Set-ItemProperty -Path $regPath -Name "Docker Desktop" -Value $exePath -ErrorAction SilentlyContinue',
    'Write-Host "Docker Desktop set to auto-start on login."'
)

$localTemp  = [System.IO.Path]::ChangeExtension([System.IO.Path]::GetTempFileName(), ".ps1")
$remotePath = "C:\tmp\__register_docker_task.ps1"

$taskScript | Set-Content -Path $localTemp -Encoding UTF8

try {
    ssh $RemoteHost "powershell -Command New-Item -ItemType Directory -Force -Path C:\tmp | Out-Null" 2>&1 | Out-Null
    scp $localTemp "${RemoteHost}:${remotePath}"
    if ($LASTEXITCODE -ne 0) { throw "scp failed" }
    ssh $RemoteHost "powershell -ExecutionPolicy Bypass -File $remotePath"
    ssh $RemoteHost "powershell -Command Remove-Item -Force $remotePath" 2>&1 | Out-Null
}
finally {
    Remove-Item $localTemp -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Setup complete." -ForegroundColor Green
Write-Host "To start Docker Desktop on $RemoteHost from SSH, run:" -ForegroundColor Cyan
Write-Host "  ssh $RemoteHost 'schtasks /run /tn LaserTargets-StartDocker'"
Write-Host ""
Write-Host "This is now done automatically by restart-remote-docker.ps1." -ForegroundColor DarkGray
