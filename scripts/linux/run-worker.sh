#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
# shellcheck source=load-env.sh
source "$script_dir/load-env.sh"
load_akasha_env "$project_root/.env"

# 顺序运行本地 worker 任务
run_worker() {
  local script="$1"

  echo "Starting ${script}"
  bun run "$script"
}

cd "$project_root/worker"
run_worker news
run_worker char
run_worker event
