$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$dataDirectory = [IO.Path]::GetFullPath(
    (Join-Path $projectRoot "data")
)

# 创建本地 SQLite 数据目录，数据库文件由后端按需创建
New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
Write-Host "SQLite data directory is ready: $dataDirectory"
