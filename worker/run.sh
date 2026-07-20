#!/bin/sh
set -u

WORKER_DIR="/app/worker"
INTERVAL_SECONDS="${WORKER_INTERVAL_SECONDS:-3600}"
BUN_INSTALL_DIR="${BUN_INSTALL:-/root/.bun}"
DEPENDENCY_RETRY_SECONDS="${DEPENDENCY_RETRY_SECONDS:-60}"
DEPENDENCY_DIR="node_modules"
DEPENDENCY_MARKER="${DEPENDENCY_DIR}/.akasha-bun-lock"

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

# 确保依赖完整且与当前锁文件一致
ensure_dependencies() {
  lock_hash="$(sha256sum bun.lock | awk '{print $1}')"

  if [ -f "$DEPENDENCY_MARKER" ] && [ "$(cat "$DEPENDENCY_MARKER")" = "$lock_hash" ]; then
    log "Worker dependencies already installed"
    return
  fi

  # 清理镜像中或上次失败后留下的不完整依赖
  rm -rf "$DEPENDENCY_DIR"

  while true; do
    log "Installing worker dependencies"

    if bun install --frozen-lockfile --production; then
      printf '%s\n' "$lock_hash" > "$DEPENDENCY_MARKER"
      return
    fi

    # 安装失败后不启动任务，等待网络恢复再重新安装
    rm -rf "$DEPENDENCY_DIR"
    log "Failed to install worker dependencies, retrying in ${DEPENDENCY_RETRY_SECONDS} seconds"
    sleep "$DEPENDENCY_RETRY_SECONDS"
  done
}

export BUN_INSTALL="$BUN_INSTALL_DIR"
export PATH="$BUN_INSTALL/bin:$PATH"

if ! command -v bun >/dev/null 2>&1; then
  log "Installing Bun runtime"
  apk add --no-cache bash curl unzip libstdc++
  curl -fsSL https://bun.sh/install | bash
else
  log "Bun runtime already installed"
fi

cd "$WORKER_DIR"
ensure_dependencies

log "Akasha worker container started"

while true; do
  run_worker "news" "news"
  run_worker "character" "char"
  run_worker "event" "event"

  log "All workers finished, reclaiming cache"
  reclaim_memory
  log "All workers finished, sleeping ${INTERVAL_SECONDS} seconds"
  sleep "${INTERVAL_SECONDS}"
done
