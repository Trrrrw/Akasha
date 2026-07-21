#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
dist_root="$project_root/dist"
worker_root="$project_root/worker"

# 复制目录到发行包中的目标位置
copy_package_directory() {
  local source="$1"
  local destination="$2"

  mkdir -p "$(dirname -- "$destination")"
  cp -a "$source" "$destination"
}

rm -rf "$dist_root"
mkdir -p "$dist_root"

# 编译后端与两个前端发布产物
(cd "$project_root" && cargo build --release -p akasha-backend)
(cd "$project_root/frontend/admin" && bun run build)
(cd "$project_root/frontend/wiki" && bun run build)

# 安装 worker 的运行时依赖
(cd "$worker_root" && bun install --frozen-lockfile --production)

# 组装后端与 worker 的运行时文件
cp "$project_root/target/release/akasha-backend" "$dist_root/akasha-backend"
copy_package_directory "$project_root/assets" "$dist_root/assets"
copy_package_directory "$project_root/frontend/admin/dist" "$dist_root/frontend/admin/dist"
copy_package_directory "$project_root/frontend/wiki/dist" "$dist_root/frontend/wiki/dist"
cp "$project_root/.env.example" "$dist_root/.env.example"
copy_package_directory "$worker_root/src" "$dist_root/worker/src"
copy_package_directory "$worker_root/node_modules" "$dist_root/worker/node_modules"
cp "$worker_root/package.json" "$dist_root/worker/package.json"
cp "$worker_root/bun.lock" "$dist_root/worker/bun.lock"
cp "$worker_root/bunfig.toml" "$dist_root/worker/bunfig.toml"
cp "$worker_root/run.sh" "$dist_root/worker/run.sh"
chmod +x "$dist_root/worker/run.sh"

echo "Created Linux package: $dist_root"
