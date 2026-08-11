param(
    [string]$RemoteHost = "",

    [ValidateSet("rpi-dev", "release")]
    [string]$Profile = "rpi-dev",

    [string]$BaseImageTag = "lasertargets-cross-aarch64",

    [string]$ArtifactImageTag = "lasertargets-server-rpi4-artifact:remote",

    [ValidateSet("auto", "plain")]
    [string]$BuildProgress = "auto",

    [bool]$NoCache = $false,

    [bool]$ExportArtifact = $true,

    [string]$LocalArtifactDir = ".\\dist\\pi",

    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path (Join-Path $scriptRoot "..")

if ($RemoteHost) {
    $env:DOCKER_HOST = "ssh://$RemoteHost"
    Write-Host "Using remote Docker host: $RemoteHost"
}

try {
    . (Join-Path $scriptRoot "docker-build-common.ps1")

    Invoke-RemoteDockerBuild `
        -Dockerfile "docker/Dockerfile.aarch64" `
        -ImageTag $BaseImageTag `
        -BuildProgress $BuildProgress `
        -NoCache $NoCache `
        -BuildContext $projectRoot `
        -DryRun $DryRun
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $buildArgs = @(
        "--build-arg", "BASE_IMAGE=$BaseImageTag",
        "--build-arg", "TARGET_TRIPLE=aarch64-unknown-linux-gnu",
        "--build-arg", "CARGO_PROFILE=$Profile"
    )

    Invoke-RemoteDockerBuild `
        -Dockerfile "docker/Dockerfile.rpi4" `
        -ImageTag $ArtifactImageTag `
        -BuildArgs $buildArgs `
        -BuildProgress $BuildProgress `
        -NoCache $NoCache `
        -BuildContext $projectRoot `
        -DryRun $DryRun
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    if (-not $DryRun -and $ExportArtifact) {
        New-Item -ItemType Directory -Force -Path $LocalArtifactDir | Out-Null
        Export-ImageArtifact -ImageTag $ArtifactImageTag -ContainerArtifactPath "/dist/server" -LocalArtifactPath (Join-Path $LocalArtifactDir "server")
        Export-ImageArtifact -ImageTag $ArtifactImageTag -ContainerArtifactPath "/dist/dac-test" -LocalArtifactPath (Join-Path $LocalArtifactDir "dac-test")
        Export-ImageArtifact -ImageTag $ArtifactImageTag -ContainerArtifactPath "/dist/libHeliosLaserDAC.so" -LocalArtifactPath (Join-Path $LocalArtifactDir "libHeliosLaserDAC.so")
    }
}
finally {
    if ($RemoteHost) {
        Remove-Item Env:DOCKER_HOST -ErrorAction SilentlyContinue
    }
}
