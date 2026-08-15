[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$BackupPath = "",

    [Parameter(Position = 1)]
    [string]$SqlitePath = "data/akasha.sqlite",

    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$importId = [Guid]::NewGuid().ToString("N")
$temporaryDirectory = Join-Path `
    ([System.IO.Path]::GetTempPath()) `
    ("akasha-postgres-import-" + $importId)
$containerName = "akasha-pg-import-$importId"
$containerBackupPath = "/tmp/akasha-backup.dump"
$containerCreated = $false
$temporarySqlitePath = $null

function Write-Step
{
    param([string]$Message)

    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Invoke-Checked
{
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [switch]$SuppressOutput
    )

    if ($SuppressOutput)
    {
        & $FilePath @Arguments *> $null
    } else
    {
        & $FilePath @Arguments
    }

    if ($LASTEXITCODE -ne 0)
    {
        throw "命令执行失败：$FilePath，退出码：$LASTEXITCODE"
    }
}

function Get-DownloadsDirectory
{
    $userShellFoldersKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders"
    $downloadsProperty = "{374DE290-123F-4565-9164-39C4925E467B}"
    $configuredPath = (Get-ItemProperty `
        -Path $userShellFoldersKey `
        -Name $downloadsProperty `
        -ErrorAction SilentlyContinue).$downloadsProperty

    if ([string]::IsNullOrWhiteSpace($configuredPath))
    {
        return Join-Path $env:USERPROFILE "Downloads"
    }

    return [Environment]::ExpandEnvironmentVariables($configuredPath)
}

function Resolve-BackupFile
{
    param([string]$Path)

    if (-not [string]::IsNullOrWhiteSpace($Path))
    {
        $resolvedPath = Resolve-Path -LiteralPath $Path -ErrorAction Stop
        if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf))
        {
            throw "备份文件不存在：$Path"
        }

        return $resolvedPath.Path
    }

    $downloadsDirectory = Get-DownloadsDirectory
    $latestBackup = Get-ChildItem `
        -LiteralPath $downloadsDirectory `
        -Filter "Akasha_*.sql.gz" `
        -File `
        -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if (-not $latestBackup)
    {
        throw "默认下载目录中没有找到 Akasha_*.sql.gz 备份：$downloadsDirectory"
    }

    return $latestBackup.FullName
}

function Resolve-SqliteFile
{
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path))
    {
        return [System.IO.Path]::GetFullPath($Path)
    }

    return [System.IO.Path]::GetFullPath((Join-Path $projectRoot $Path))
}

function Get-FreeTcpPort
{
    $listener = [System.Net.Sockets.TcpListener]::new(
        [System.Net.IPAddress]::Loopback,
        0
    )

    try
    {
        $listener.Start()
        return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    } finally
    {
        $listener.Stop()
    }
}

function Remove-SqliteArtifacts
{
    param([string]$Path)

    foreach ($suffix in @("", "-wal", "-shm", "-journal"))
    {
        $artifactPath = "$Path$suffix"
        if (Test-Path -LiteralPath $artifactPath)
        {
            Remove-Item -LiteralPath $artifactPath -Force -ErrorAction SilentlyContinue
        }
    }
}

$backupFile = Resolve-BackupFile $BackupPath
$resolvedSqlitePath = Resolve-SqliteFile $SqlitePath
$sqliteDirectory = Split-Path -Parent $resolvedSqlitePath
$sqliteFileName = Split-Path -Leaf $resolvedSqlitePath
$temporarySqlitePath = Join-Path $sqliteDirectory (".$sqliteFileName.import-$importId")

if (-not (Get-Command docker.exe -ErrorAction SilentlyContinue))
{
    throw "找不到 docker.exe"
}

if (-not (Get-Command cargo.exe -ErrorAction SilentlyContinue))
{
    throw "找不到 cargo.exe"
}

$sevenZipCommand = Get-Command 7z.exe -ErrorAction SilentlyContinue
if (-not $sevenZipCommand)
{
    $sevenZipCommand = Get-Command 7z -ErrorAction SilentlyContinue
}
if (-not $sevenZipCommand)
{
    throw "找不到 7-Zip，请确认 7z.exe 已加入 PATH"
}

New-Item -ItemType Directory -Path $sqliteDirectory -Force | Out-Null
New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null

$targetArtifacts = @(
    @("", "-wal", "-shm", "-journal") |
    ForEach-Object { "$resolvedSqlitePath$_" } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf }
)
$targetExists = $targetArtifacts.Count -gt 0
if ($targetExists -and -not $Force)
{
    $confirmation = Read-Host "目标 SQLite 已存在，导入完成后会替换它，继续吗？输入 yes 确认"
    if ($confirmation -ne "yes")
    {
        throw "已取消导入"
    }
}

$postgresUser = "akasha_import"
$postgresPassword = "Import$importId"
$postgresDatabase = "Akasha"
$postgresPort = Get-FreeTcpPort
$postgresUrl = "postgres://${postgresUser}:${postgresPassword}@127.0.0.1:${postgresPort}/${postgresDatabase}"
$previousImportUrl = [Environment]::GetEnvironmentVariable(
    "AKASHA_IMPORT_POSTGRES_URL",
    "Process"
)

try
{
    Write-Step "解压 PostgreSQL 备份"
    Invoke-Checked `
        $sevenZipCommand.Source `
        @(
            "x",
            "-y",
            "-o$temporaryDirectory",
            "--",
            $backupFile
        ) `
        -SuppressOutput

    $dumpFile = Get-ChildItem `
        -LiteralPath $temporaryDirectory `
        -Recurse `
        -File |
        Sort-Object Length -Descending |
        Select-Object -First 1
    if (-not $dumpFile)
    {
        throw "备份解压后没有找到数据库文件"
    }

    Write-Host "备份文件：$([System.IO.Path]::GetFileName($backupFile))"
    Write-Host "备份大小：$([Math]::Round($dumpFile.Length / 1MB, 2)) MB"

    Write-Step "启动临时 PostgreSQL 容器"
    Invoke-Checked "docker.exe" @(
        "run",
        "-d",
        "--name", $containerName,
        "--publish", "127.0.0.1:${postgresPort}:5432",
        "--env", "POSTGRES_USER=$postgresUser",
        "--env", "POSTGRES_PASSWORD=$postgresPassword",
        "--env", "POSTGRES_DB=$postgresDatabase",
        "postgres:18.4-alpine"
    ) -SuppressOutput
    $containerCreated = $true

    Write-Step "等待临时 PostgreSQL 就绪"
    $postgresReady = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++)
    {
        & docker.exe exec $containerName pg_isready -U $postgresUser -d $postgresDatabase *> $null
        if ($LASTEXITCODE -eq 0)
        {
            $postgresReady = $true
            break
        }

        Start-Sleep -Seconds 1
    }
    if (-not $postgresReady)
    {
        throw "临时 PostgreSQL 在 60 秒内没有就绪"
    }

    Write-Step "恢复备份到临时 PostgreSQL"
    Invoke-Checked "docker.exe" @(
        "cp",
        $dumpFile.FullName,
        "${containerName}:$containerBackupPath"
    ) -SuppressOutput

    & docker.exe exec $containerName pg_restore --list $containerBackupPath *> $null
    $isArchiveFormat = $LASTEXITCODE -eq 0
    if ($isArchiveFormat)
    {
        Invoke-Checked "docker.exe" @(
            "exec",
            $containerName,
            "pg_restore",
            "--exit-on-error",
            "--no-owner",
            "--no-privileges",
            "-U", $postgresUser,
            "-d", $postgresDatabase,
            $containerBackupPath
        )
    } else
    {
        Invoke-Checked "docker.exe" @(
            "exec",
            $containerName,
            "psql",
            "-v", "ON_ERROR_STOP=1",
            "-U", $postgresUser,
            "-d", $postgresDatabase,
            "-f", $containerBackupPath
        )
    }

    Write-Step "分页导入 SQLite 临时文件"
    $env:AKASHA_IMPORT_POSTGRES_URL = $postgresUrl
    try
    {
        Invoke-Checked "cargo.exe" @(
            "run",
            "--locked",
            "-p", "akasha-db",
            "--features", "postgres-import",
            "--bin", "import-postgres",
            "--",
            "--sqlite-path", $temporarySqlitePath
        )
    } finally
    {
        if ($null -eq $previousImportUrl)
        {
            Remove-Item Env:AKASHA_IMPORT_POSTGRES_URL -ErrorAction SilentlyContinue
        } else
        {
            $env:AKASHA_IMPORT_POSTGRES_URL = $previousImportUrl
        }
    }

    Write-Step "替换 SQLite 数据库文件"
    $existingBackupDirectory = $null
    try
    {
        if ($targetExists)
        {
            $timestamp = Get-Date -Format "yyyyMMddHHmmss"
            $existingBackupDirectory = "$resolvedSqlitePath.before-import-$timestamp"
            New-Item -ItemType Directory -Path $existingBackupDirectory -Force | Out-Null

            foreach ($artifactPath in $targetArtifacts)
            {
                Move-Item `
                    -LiteralPath $artifactPath `
                    -Destination (Join-Path $existingBackupDirectory (Split-Path -Leaf $artifactPath))
            }
        }

        Move-Item -LiteralPath $temporarySqlitePath -Destination $resolvedSqlitePath
        Write-Host "SQLite 导入完成：$resolvedSqlitePath" -ForegroundColor Green
        if ($existingBackupDirectory)
        {
            Write-Host "旧 SQLite 已保留在：$existingBackupDirectory"
        }
    } catch
    {
        if ($existingBackupDirectory -and (Test-Path -LiteralPath $existingBackupDirectory))
        {
            foreach ($artifactName in @(
                    (Split-Path -Leaf $resolvedSqlitePath),
                    (Split-Path -Leaf "$resolvedSqlitePath-wal"),
                    (Split-Path -Leaf "$resolvedSqlitePath-shm"),
                    (Split-Path -Leaf "$resolvedSqlitePath-journal")
                ))
            {
                $backupArtifactPath = Join-Path $existingBackupDirectory $artifactName
                $originalArtifactPath = Join-Path $sqliteDirectory $artifactName
                if (Test-Path -LiteralPath $backupArtifactPath)
                {
                    Move-Item -LiteralPath $backupArtifactPath -Destination $originalArtifactPath
                }
            }

            Remove-Item -LiteralPath $existingBackupDirectory -Force -ErrorAction SilentlyContinue
        }

        throw
    }
} finally
{
    if ($containerCreated)
    {
        & docker.exe rm -f $containerName *> $null
    }

    if (Test-Path -LiteralPath $temporaryDirectory)
    {
        Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force -ErrorAction SilentlyContinue
    }

    if ($temporarySqlitePath)
    {
        Remove-SqliteArtifacts $temporarySqlitePath
    }
}
