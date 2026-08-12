$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$distRoot = Join-Path $projectRoot "dist"

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

# 编译后端发布产物
Invoke-ExternalCommand "cargo.exe" @("build", "--release", "-p", "akasha-backend") $projectRoot

# 组装后端运行时文件
Copy-Item `
    -LiteralPath (Join-Path $projectRoot "target/release/akasha-backend.exe") `
    -Destination (Join-Path $distRoot "akasha-backend.exe") `
    -Force
Copy-PackageDirectory (Join-Path $projectRoot "assets") (Join-Path $distRoot "assets")
New-Item -ItemType Directory -Path (Join-Path $distRoot "config") -Force | Out-Null
Copy-Item `
    -LiteralPath (Join-Path $projectRoot "config/backend.toml.example") `
    -Destination (Join-Path $distRoot "config/backend.toml.example") `
    -Force
Copy-Item -LiteralPath (Join-Path $projectRoot ".env.example") -Destination (Join-Path $distRoot ".env.example") -Force

Write-Host "Created Windows package: $distRoot"
