#!/bin/sh
set -u

# 容器与发行包都以脚本所在目录作为默认工作目录
WORKER_DIR="${WORKER_DIR:-$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)}"
INTERVAL_SECONDS="${WORKER_INTERVAL_SECONDS:-3600}"

log() {
  echo "[$(date '+%Y-%m-%dT%H:%M:%S%z')] $1"
}

run_worker() {
  name="$1"
  script="$2"

  log "Starting ${name}"

  if bun run "$script"; then
    log "Finished ${name}"
  else
    status=$?
    log "Failed ${name}, exit code: ${status}"
  fi
}

reclaim_memory() {
  printf '%s\n' '128M' > /sys/fs/cgroup/memory.reclaim 2>/dev/null || true
}

cd "$WORKER_DIR"

log "Akasha worker container started"

while true; do
  run_worker "news" "news"
  run_worker "character" "char"
  run_worker "event" "event"

  log "All workers finished, reclaiming memory"
  reclaim_memory
  log "All workers finished, sleeping ${INTERVAL_SECONDS} seconds"
  sleep "${INTERVAL_SECONDS}"
done
