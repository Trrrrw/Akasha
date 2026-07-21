#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
# shellcheck source=load-env.sh
source "$script_dir/load-env.sh"
load_akasha_env "$project_root/.env"

# 创建或启动本地 PostgreSQL 开发容器
start_akasha_database() {
  : "${POSTGRES_PORT:?POSTGRES_PORT is required}"
  : "${POSTGRES_USER:?POSTGRES_USER is required}"
  : "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}"
  : "${POSTGRES_DB:?POSTGRES_DB is required}"

  local container_name="akasha-db-dev"
  local data_directory="$project_root/.temp/postgresql"
  mkdir -p "$data_directory"

  if docker container inspect "$container_name" >/dev/null 2>&1; then
    if [[ "$(docker inspect --format '{{.State.Status}}' "$container_name")" == "running" ]]; then
      echo "PostgreSQL container is already running"
      return
    fi

    docker start "$container_name"
    return
  fi

  docker run -d \
    --name "$container_name" \
    --restart unless-stopped \
    --publish "127.0.0.1:${POSTGRES_PORT}:5432" \
    --env "POSTGRES_USER=${POSTGRES_USER}" \
    --env "POSTGRES_PASSWORD=${POSTGRES_PASSWORD}" \
    --env "POSTGRES_DB=${POSTGRES_DB}" \
    --mount "type=bind,src=${data_directory},dst=/var/lib/postgresql" \
    postgres:18.4-alpine
}

start_akasha_database
