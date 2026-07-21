# 将 .env 文件中的键值加载到当前进程环境
function Import-AkashaEnvironment {
    param([string] $path)

    if (-not (Test-Path -LiteralPath $path)) {
        return
    }

    foreach ($line in Get-Content -LiteralPath $path) {
        $trimmed = $line.Trim()

        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) {
            continue
        }

        $pair = $trimmed -split "=", 2
        if ($pair.Count -ne 2) {
            throw "Invalid environment entry: $line"
        }

        $key = $pair[0].Trim()
        $value = $pair[1].Trim()

        if (
            $value.Length -ge 2 -and
            (($value.StartsWith('"') -and $value.EndsWith('"')) -or
                ($value.StartsWith("'") -and $value.EndsWith("'")))
        ) {
            $value = $value.Substring(1, $value.Length - 2)
        }

        [Environment]::SetEnvironmentVariable($key, $value, "Process")
    }
}
