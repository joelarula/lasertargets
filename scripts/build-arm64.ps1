#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build LaserTargets for Raspberry Pi 4 (aarch64) using Dockerfile.rpi4 on the remote build server.

.DESCRIPTION
    Packages the local workspace source into a temporary tarball, transfers it to the
    dedicated remote Docker build host (joel@192.168.1.110), builds the artifact image
    using docker/Dockerfile.rpi4, extracts the /dist binaries via docker create/cp,
    and retrieves them into local dist\pi\.

.PARAMETER RemoteHost
    SSH target of the remote Docker build server. Defaults to 'joel@192.168.1.110'.

.PARAMETER Profile
    Cargo profile: 'rpi-dev' or 'release'. Defaults to 'rpi-dev'.

.PARAMETER NoCache
    Do not use Docker layer cache.

.PARAMETER LocalArtifactDir
    Destination folder for compiled ARM64 binaries. Defaults to 'dist\pi'.

.PARAMETER DryRun
    Print commands without executing them.

.EXAMPLE
    .\scripts\build-arm64.ps1
    .\scripts\build-arm64.ps1 -Profile release
#>
param(
    [string]$RemoteHost = "joel@192.168.1.110",

    [ValidateSet("rpi-dev", "release")]
    [string]$Profile = "rpi-dev",

    [switch]$NoCache,

    [string]$LocalArtifactDir = "dist\pi",

    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path (Join-Path $scriptRoot "..")

Write-Host "=== LaserTargets Remote Docker Build (ARM64) ===" -ForegroundColor Cyan
Write-Host "Build Server : $RemoteHost" -ForegroundColor Gray
Write-Host "Dockerfile   : docker/Dockerfile.rpi4" -ForegroundColor Gray
Write-Host "Profile      : $Profile" -ForegroundColor Gray
Write-Host "Output       : $LocalArtifactDir" -ForegroundColor Gray

if ($DryRun) {
    Write-Host "[DRY-RUN] Would tar source, scp to $RemoteHost, run docker build -f docker/Dockerfile.rpi4, and extract artifacts." -ForegroundColor Yellow
    exit 0
}

# 1. Package source context into a local temp tarball
Write-Host "`n[1/4] Packaging source code..." -ForegroundColor Cyan
$tarFile = Join-Path $env:TEMP "lasertargets_src.tar"
if (Test-Path $tarFile) { Remove-Item $tarFile -Force -ErrorAction SilentlyContinue }

& tar --exclude=target --exclude=dist --exclude=.git --exclude=*.so --exclude=*.exe -cf $tarFile -C $projectRoot .
if ($LASTEXITCODE -ne 0) {
    throw "Failed to create source tarball."
}

# 2. Transfer tarball to remote Docker build host
Write-Host "`n[2/4] Syncing source to remote Docker host ($RemoteHost)..." -ForegroundColor Cyan
$remoteTar = "C:\Users\joel\AppData\Local\Temp\lasertargets_src.tar"
& scp $tarFile "${RemoteHost}:${remoteTar}"
if ($LASTEXITCODE -ne 0) {
    Remove-Item $tarFile -Force -ErrorAction SilentlyContinue
    throw "Failed to transfer source tarball to $RemoteHost."
}
Remove-Item $tarFile -Force -ErrorAction SilentlyContinue

# 3. Unpack and build with Dockerfile.rpi4 on the remote build server
Write-Host "`n[3/4] Building Docker image with docker/Dockerfile.rpi4 on $RemoteHost..." -ForegroundColor Cyan

$remoteBuildDir = "C:\Users\joel\AppData\Local\Temp\rpi_build"
$noCacheArg = if ($NoCache) { "--no-cache" } else { "" }

$remoteScript = @"
powershell -Command "
    `$ErrorActionPreference = 'Stop'
    Remove-Item -Recurse -Force $remoteBuildDir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $remoteBuildDir\dist | Out-Null
    Set-Location $remoteBuildDir
    tar -xf $remoteTar
    Remove-Item $remoteTar -Force -ErrorAction SilentlyContinue

    `$env:DOCKER_BUILDKIT = '1'
    Write-Host '--> docker build with docker/Dockerfile.rpi4...' -ForegroundColor Cyan
    docker build $noCacheArg -f docker/Dockerfile.rpi4 -t lasertargets-server-rpi4-artifact:remote --build-arg BASE_IMAGE=lasertargets-cross-aarch64 --build-arg TARGET_TRIPLE=aarch64-unknown-linux-gnu --build-arg CARGO_PROFILE=$Profile .
    if (`$LASTEXITCODE -ne 0) { exit `$LASTEXITCODE }

    Write-Host '--> Extracting artifacts from container...' -ForegroundColor Cyan
    `$cid = docker create lasertargets-server-rpi4-artifact:remote
    try {
        docker cp `"`${cid}:/dist/.`" $remoteBuildDir\dist\
    } finally {
        docker rm -f `$cid | Out-Null
    }
"
"@

& ssh -t $RemoteHost $remoteScript
if ($LASTEXITCODE -ne 0) {
    throw "Docker build failed on remote server."
}

# 4. Fetch compiled artifacts back to local workspace
Write-Host "`n[4/4] Fetching compiled ARM64 artifacts..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $LocalArtifactDir | Out-Null

$remoteDist = "$remoteBuildDir\dist"
& scp "${RemoteHost}:${remoteDist}\server" (Join-Path $LocalArtifactDir "server")
& scp "${RemoteHost}:${remoteDist}\dac-test" (Join-Path $LocalArtifactDir "dac-test") 2>$null
& scp "${RemoteHost}:${remoteDist}\libHeliosLaserDAC.so" (Join-Path $LocalArtifactDir "libHeliosLaserDAC.so") 2>$null

Write-Host "`n=== Build Complete ===" -ForegroundColor Green
Write-Host "Artifacts ready in: $(Resolve-Path $LocalArtifactDir)" -ForegroundColor Green
