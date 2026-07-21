$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $PSScriptRoot "load-env.ps1")
Import-AkashaEnvironment (Join-Path $projectRoot ".env")

# 启动一个开发进程并保留其输出在当前终端
function Start-DevelopmentProcess {
    param(
        [string] $filePath,
        [string[]] $argumentList,
        [string] $workingDirectory
    )

    Start-Process `
        -FilePath $filePath `
        -ArgumentList $argumentList `
        -WorkingDirectory $workingDirectory `
        -NoNewWindow `
        -PassThru
}

# 终止指定进程及其所有子进程
function Stop-ProcessTree {
    param([System.Diagnostics.Process] $process)

    if (-not $process.HasExited) {
        & taskkill.exe /PID $process.Id /T /F *> $null
    }
}

$processes = @(
    Start-DevelopmentProcess "cargo.exe" @("run", "-p", "akasha-backend") $projectRoot
    Start-DevelopmentProcess "bun.exe" @("run", "dev") (Join-Path $projectRoot "frontend/admin")
    Start-DevelopmentProcess "bun.exe" @("run", "dev") (Join-Path $projectRoot "frontend/wiki")
)

try {
    while ($true) {
        foreach ($process in $processes) {
            $process.Refresh()
        }

        $completed = $processes | Where-Object { $_.HasExited } | Select-Object -First 1
        if ($null -ne $completed) {
            exit $completed.ExitCode
        }

        Start-Sleep -Seconds 1
    }
}
finally {
    $processes | ForEach-Object { Stop-ProcessTree $_ }
}
