$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$workerRoot = Join-Path $projectRoot "worker"
$intervalSeconds = if ($env:WORKER_INTERVAL_SECONDS) {
    [int] $env:WORKER_INTERVAL_SECONDS
}
else {
    3600
}
. (Join-Path $PSScriptRoot "load-env.ps1")
Import-AkashaEnvironment (Join-Path $projectRoot ".env")

# 输出 worker 的带时间戳日志
function Write-WorkerLog {
    param([string] $message)

    $time = Get-Date -Format "yyyy-MM-ddTHH:mm:ssK"
    Write-Host "[$time] $message"
}

# 运行一个 worker 任务并记录结果
function Invoke-Worker {
    param(
        [string] $name,
        [string] $script
    )

    Write-WorkerLog "Starting $name"

    try {
        & bun.exe run $script
        $exitCode = $LASTEXITCODE
    }
    catch {
        Write-WorkerLog "Failed $($name): $($_)"
        return
    }

    if ($exitCode -eq 0) {
        Write-WorkerLog "Finished $name"
    }
    else {
        Write-WorkerLog "Failed $name, exit code: $exitCode"
    }
}

Push-Location $workerRoot
try {
    Write-WorkerLog "Akasha worker started"

    while ($true) {
        Invoke-Worker "news" "news"
        Invoke-Worker "character" "char"
        Invoke-Worker "event" "event"

        Write-WorkerLog "All workers finished, sleeping $intervalSeconds seconds"
        Start-Sleep -Seconds $intervalSeconds
    }
}
finally {
    Pop-Location
}
