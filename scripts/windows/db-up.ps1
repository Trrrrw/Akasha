$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $PSScriptRoot "load-env.ps1")
Import-AkashaEnvironment (Join-Path $projectRoot ".env")

# 创建或启动本地 PostgreSQL 开发容器
function Start-AkashaDatabase {
    $requiredVariables = @(
        "POSTGRES_PORT",
        "POSTGRES_USER",
        "POSTGRES_PASSWORD",
        "POSTGRES_DB"
    )
    $missingVariables = $requiredVariables | Where-Object {
        [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($_))
    }

    if ($missingVariables) {
        throw "Missing required environment variables: $($missingVariables -join ', ')"
    }

    $containerName = "akasha-db-dev"
    $dataDirectory = [IO.Path]::GetFullPath(
        (Join-Path $projectRoot ".temp/postgresql")
    )
    New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null

    & docker.exe container inspect $containerName *> $null
    $containerExists = $LASTEXITCODE -eq 0

    if ($containerExists) {
        $status = & docker.exe inspect --format "{{.State.Status}}" $containerName
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to inspect $containerName"
        }

        if ($status -eq "running") {
            Write-Host "PostgreSQL container is already running"
            return
        }

        & docker.exe start $containerName
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to start $containerName"
        }

        return
    }

    & docker.exe run -d `
        --name $containerName `
        --restart unless-stopped `
        --publish "127.0.0.1:$($env:POSTGRES_PORT):5432" `
        --env "POSTGRES_USER=$($env:POSTGRES_USER)" `
        --env "POSTGRES_PASSWORD=$($env:POSTGRES_PASSWORD)" `
        --env "POSTGRES_DB=$($env:POSTGRES_DB)" `
        --mount "type=bind,src=$dataDirectory,dst=/var/lib/postgresql" `
        postgres:18.4-alpine

    if ($LASTEXITCODE -ne 0) {
        throw "Failed to create $containerName"
    }
}

Start-AkashaDatabase
