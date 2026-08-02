function Invoke-RemoteDockerBuild {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Dockerfile,

        [Parameter(Mandatory = $true)]
        [string]$ImageTag,

        [string[]]$BuildArgs = @(),

        [string]$BuildContext = ".",

        [ValidateSet("auto", "plain")]
        [string]$BuildProgress = "auto",

        [bool]$NoCache = $false,

        [bool]$DryRun = $false
    )

    $commonBuildArgs = @("--progress", $BuildProgress)
    if ($NoCache) {
        $commonBuildArgs += "--no-cache"
    }

    $buildCommandPreview = "docker build " + (($commonBuildArgs + $BuildArgs + @("-f", $Dockerfile, "-t", $ImageTag, $BuildContext)) -join " ")
    Write-Host "Running: $buildCommandPreview"

    if ($DryRun) {
        return 0
    }

    $env:DOCKER_BUILDKIT = "1"
    & docker build @commonBuildArgs @BuildArgs -f $Dockerfile -t $ImageTag $BuildContext
    $exitCode = $LASTEXITCODE
    Remove-Item Env:\DOCKER_BUILDKIT -ErrorAction SilentlyContinue
    return $exitCode
}

function Export-ImageArtifact {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ImageTag,

        [Parameter(Mandatory = $true)]
        [string]$ContainerArtifactPath,

        [Parameter(Mandatory = $true)]
        [string]$LocalArtifactPath
    )

    $dir = Split-Path -Parent $LocalArtifactPath
    if ($dir) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }

    $cid = docker create $ImageTag
    if (-not $cid) {
        throw "Failed to create container from image $ImageTag"
    }

    try {
        & docker cp "${cid}:$ContainerArtifactPath" $LocalArtifactPath
        if ($LASTEXITCODE -ne 0) {
            throw "docker cp failed for ${cid}:$ContainerArtifactPath"
        }
    }
    finally {
        & docker rm $cid | Out-Null
    }

    Write-Host "Exported artifact to: $LocalArtifactPath"
}
