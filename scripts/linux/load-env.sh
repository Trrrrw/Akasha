#!/usr/bin/env bash

# 将 .env 文件中的键值导出到当前 shell
load_akasha_env() {
  local path="$1"

  [[ -f "$path" ]] || return 0

  set -a
  # shellcheck disable=SC1090
  source "$path"
  set +a
}
