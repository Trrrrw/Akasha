[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateScript({
            if (-not (Test-Path -LiteralPath $_ -PathType Leaf))
            {
                throw "备份文件不存在：$_"
            }
            $true
        })]
    [string]$BackupGz,

    [Parameter(Mandatory = $true, Position = 1)]
    [ValidateNotNullOrEmpty()]
    [string]$ContainerName
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step
{
    param([string]$Message)
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Get-RequiredEnvironmentVariable
{
    param([string]$Name)

    $Value = [Environment]::GetEnvironmentVariable($Name)

    if ([string]::IsNullOrWhiteSpace($Value))
    {
        throw "缺少环境变量：$Name"
    }

    return $Value
}

function Invoke-Docker
{
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [switch]$AllowFailure,

        [switch]$SuppressOutput,

        [switch]$PassThru
    )

    if ($SuppressOutput)
    {
        & docker @Arguments *> $null
    } else
    {
        & docker @Arguments
    }

    $Code = $LASTEXITCODE

    if (-not $AllowFailure -and $Code -ne 0)
    {
        # 参数可能包含 PGPASSWORD，错误中不能回显完整命令
        throw "Docker 命令执行失败，退出码：$Code"
    }

    if ($PassThru)
    {
        return $Code
    }
}

$PostgresUser = Get-RequiredEnvironmentVariable "POSTGRES_USER"
$PostgresPassword = Get-RequiredEnvironmentVariable "POSTGRES_PASSWORD"
$PostgresDatabase = Get-RequiredEnvironmentVariable "POSTGRES_DB"

if (-not (Get-Command docker -ErrorAction SilentlyContinue))
{
    throw "找不到 docker 命令。"
}

$SevenZipCommand = Get-Command 7z.exe -ErrorAction SilentlyContinue

if (-not $SevenZipCommand)
{
    $SevenZipCommand = Get-Command 7z -ErrorAction SilentlyContinue
}

if (-not $SevenZipCommand)
{
    throw "找不到 7-Zip。请确认 7z.exe 已加入 PATH。"
}

$ResolvedBackupPath = (Resolve-Path -LiteralPath $BackupGz).Path
$TemporaryDirectory = Join-Path `
([System.IO.Path]::GetTempPath()) `
("onepanel-postgres-restore-" + [Guid]::NewGuid().ToString("N"))

$ContainerBackupPath = "/tmp/onepanel-postgres-restore.dump"

try
{
    Write-Step "检查 PostgreSQL 容器"

    $RunningContainer = & docker inspect `
        --format "{{.State.Running}}" `
        $ContainerName 2>$null

    if ($LASTEXITCODE -ne 0)
    {
        throw "Docker 容器不存在：$ContainerName"
    }

    if ($RunningContainer.Trim() -ne "true")
    {
        throw "Docker 容器尚未运行：$ContainerName"
    }

    Write-Step "检查容器中的 PostgreSQL 工具"

    Invoke-Docker -Arguments @(
        "exec",
        $ContainerName,
        "psql",
        "--version"
    )

    New-Item `
        -ItemType Directory `
        -Path $TemporaryDirectory `
        -Force | Out-Null

    Write-Step "使用 7-Zip 解压备份"

    & $SevenZipCommand.Source `
        "x" `
        "-y" `
        "-o$TemporaryDirectory" `
        "--" `
        $ResolvedBackupPath

    if ($LASTEXITCODE -ne 0)
    {
        throw "7-Zip 解压失败，退出码：$LASTEXITCODE"
    }

    # 1Panel 的文件可能扩展名为 .sql，
    # 但内部实际是 PostgreSQL custom-format dump。
    # 一般最大的解压文件就是数据库备份。
    $DumpFile = Get-ChildItem `
        -LiteralPath $TemporaryDirectory `
        -Recurse `
        -File |
        Where-Object {
            $_.Length -gt 0 -and
            $_.Extension.ToLowerInvariant() -notin @(
                ".gz",
                ".zip",
                ".7z",
                ".tar"
            )
        } |
        Sort-Object Length -Descending |
        Select-Object -First 1

    if (-not $DumpFile)
    {
        throw "解压后没有找到数据库备份文件。"
    }

    Write-Host "备份文件：$($DumpFile.FullName)"
    Write-Host "文件大小：$([Math]::Round($DumpFile.Length / 1MB, 2)) MB"

    Write-Step "将备份复制到 PostgreSQL 容器"

    Invoke-Docker -Arguments @(
        "cp",
        $DumpFile.FullName,
        "${ContainerName}:$ContainerBackupPath"
    )

    Write-Step "检测备份格式"

    $FormatCheckCode = Invoke-Docker `
        -AllowFailure `
        -SuppressOutput `
        -PassThru `
        -Arguments @(
        "exec",
        "-e", "PGPASSWORD=$PostgresPassword",
        $ContainerName,
        "pg_restore",
        "--list",
        $ContainerBackupPath
    )

    $IsArchiveFormat = ($FormatCheckCode -eq 0)

    if ($IsArchiveFormat)
    {
        Write-Host "备份格式：PostgreSQL custom/tar archive"
    } else
    {
        Write-Host "备份格式：纯文本 SQL"
    }

    Write-Host ""
    Write-Host "警告：即将覆盖本地数据库 '$PostgresDatabase'。" `
        -ForegroundColor Yellow

    # dropdb --force 会断开现有会话后删除数据库
    Write-Step "强制断开连接并删除原有数据库"

    Invoke-Docker -Arguments @(
        "exec",
        "-e", "PGPASSWORD=$PostgresPassword",
        $ContainerName,
        "dropdb",
        "--if-exists",
        "--force",
        "-U", $PostgresUser,
        $PostgresDatabase
    )

    Write-Step "重新创建空数据库"

    Invoke-Docker -Arguments @(
        "exec",
        "-e", "PGPASSWORD=$PostgresPassword",
        $ContainerName,
        "createdb",
        "-U", $PostgresUser,
        "-O", $PostgresUser,
        $PostgresDatabase
    )

    if ($IsArchiveFormat)
    {
        Write-Step "使用 pg_restore 恢复 custom-format 备份"

        Invoke-Docker -Arguments @(
            "exec",
            "-e", "PGPASSWORD=$PostgresPassword",
            $ContainerName,
            "pg_restore",
            "--exit-on-error",
            "--no-owner",
            "--no-privileges",
            "--verbose",
            "-U", $PostgresUser,
            "-d", $PostgresDatabase,
            $ContainerBackupPath
        )
    } else
    {
        Write-Step "使用 psql 恢复纯文本 SQL 备份"

        Invoke-Docker -Arguments @(
            "exec",
            "-e", "PGPASSWORD=$PostgresPassword",
            $ContainerName,
            "psql",
            "-v", "ON_ERROR_STOP=1",
            "-U", $PostgresUser,
            "-d", $PostgresDatabase,
            "-f", $ContainerBackupPath
        )
    }

    Write-Step "验证恢复结果"

    Invoke-Docker -Arguments @(
        "exec",
        "-e", "PGPASSWORD=$PostgresPassword",
        $ContainerName,
        "psql",
        "-v", "ON_ERROR_STOP=1",
        "-U", $PostgresUser,
        "-d", $PostgresDatabase,
        "-c", @"
SELECT
    current_database() AS database,
    pg_size_pretty(pg_database_size(current_database())) AS database_size,
    (
        SELECT count(*)
        FROM pg_catalog.pg_tables
        WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
    ) AS table_count;
"@
    )

    Write-Host "`n数据库恢复完成。" -ForegroundColor Green
} finally
{
    Write-Step "清理临时文件"

    Invoke-Docker `
        -AllowFailure `
        -SuppressOutput `
        -Arguments @(
        "exec",
        $ContainerName,
        "rm",
        "-f",
        $ContainerBackupPath
    ) | Out-Null

    if (Test-Path -LiteralPath $TemporaryDirectory)
    {
        Remove-Item `
            -LiteralPath $TemporaryDirectory `
            -Recurse `
            -Force `
            -ErrorAction SilentlyContinue
    }
}
