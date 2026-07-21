#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
# shellcheck source=load-env.sh
source "$script_dir/load-env.sh"
load_akasha_env "$project_root/.env"

declare -a process_ids=()

# 终止开发服务及其直接子进程
cleanup() {
  local process_id

  trap - EXIT INT TERM
  for process_id in "${process_ids[@]}"; do
    pkill -TERM -P "$process_id" 2>/dev/null || true
    kill "$process_id" 2>/dev/null || true
  done
}

trap cleanup EXIT INT TERM

# 启动后端与两个前端开发服务
(cd "$project_root" && cargo run -p akasha-backend) &
process_ids+=("$!")
(cd "$project_root/frontend/admin" && bun run dev) &
process_ids+=("$!")
(cd "$project_root/frontend/wiki" && bun run dev) &
process_ids+=("$!")

wait -n "${process_ids[@]}"
