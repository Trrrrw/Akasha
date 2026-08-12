#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd -- "$script_dir/../.." && pwd)"
dist_root="$project_root/dist"

# 复制目录到发行包中的目标位置
copy_package_directory() {
  local source="$1"
  local destination="$2"

  mkdir -p "$(dirname -- "$destination")"
  cp -a "$source" "$destination"
}

rm -rf "$dist_root"
mkdir -p "$dist_root"

# 编译后端发布产物
(cd "$project_root" && cargo build --release -p akasha-backend)

# 组装后端运行时文件
cp "$project_root/target/release/akasha-backend" "$dist_root/akasha-backend"
copy_package_directory "$project_root/assets" "$dist_root/assets"
mkdir -p "$dist_root/config"
cp "$project_root/config/backend.toml.example" "$dist_root/config/backend.toml.example"
cp "$project_root/.env.example" "$dist_root/.env.example"

echo "Created Linux package: $dist_root"
