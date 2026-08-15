#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
data_directory="$project_root/data"

# 创建本地 SQLite 数据目录，数据库文件由后端按需创建
mkdir -p "$data_directory"
echo "SQLite data directory is ready: $data_directory"
