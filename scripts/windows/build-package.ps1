$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$distRoot = Join-Path $projectRoot "dist"
$workerRoot = Join-Path $projectRoot "worker"

# 运行外部命令并将非零退出码转换为错误
function Invoke-ExternalCommand {
    param(
        [string] $filePath,
        [string[]] $argumentList,
        [string] $workingDirectory
    )

    Push-Location $workingDirectory
    try {
        & $filePath @argumentList
        if ($LASTEXITCODE -ne 0) {
            throw "$filePath failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        Pop-Location
    }
}

# 复制目录到发行包中的目标位置
function Copy-PackageDirectory {
    param(
        [string] $source,
        [string] $destination
    )

    New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
    Copy-Item -LiteralPath $source -Destination $destination -Recurse -Force
}

Remove-Item -LiteralPath $distRoot -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $distRoot -Force | Out-Null

# 编译后端与两个前端发布产物
Invoke-ExternalCommand "cargo.exe" @("build", "--release", "-p", "akasha-backend") $projectRoot
Invoke-ExternalCommand "bun.exe" @("run", "build") (Join-Path $projectRoot "frontend/admin")
Invoke-ExternalCommand "bun.exe" @("run", "build") (Join-Path $projectRoot "frontend/wiki")

# 安装 worker 的运行时依赖
Invoke-ExternalCommand "bun.exe" @("install", "--frozen-lockfile", "--production") $workerRoot

# 组装后端与 worker 的运行时文件
Copy-Item `
    -LiteralPath (Join-Path $projectRoot "target/release/akasha-backend.exe") `
    -Destination (Join-Path $distRoot "akasha-backend.exe") `
    -Force
Copy-PackageDirectory (Join-Path $projectRoot "assets") (Join-Path $distRoot "assets")
Copy-PackageDirectory (Join-Path $projectRoot "frontend/admin/dist") (Join-Path $distRoot "frontend/admin/dist")
Copy-PackageDirectory (Join-Path $projectRoot "frontend/wiki/dist") (Join-Path $distRoot "frontend/wiki/dist")
Copy-Item -LiteralPath (Join-Path $projectRoot ".env.example") -Destination (Join-Path $distRoot ".env.example") -Force
Copy-PackageDirectory (Join-Path $workerRoot "src") (Join-Path $distRoot "worker/src")
Copy-PackageDirectory (Join-Path $workerRoot "node_modules") (Join-Path $distRoot "worker/node_modules")
Copy-Item -LiteralPath (Join-Path $workerRoot "package.json") -Destination (Join-Path $distRoot "worker/package.json") -Force
Copy-Item -LiteralPath (Join-Path $workerRoot "bun.lock") -Destination (Join-Path $distRoot "worker/bun.lock") -Force
Copy-Item -LiteralPath (Join-Path $workerRoot "bunfig.toml") -Destination (Join-Path $distRoot "worker/bunfig.toml") -Force

Write-Host "Created Windows package: $distRoot"
