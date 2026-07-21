# --- 编译admin前端 ---
FROM oven/bun:alpine AS bun-builder
WORKDIR /app/frontend/admin
COPY frontend/admin/package.json frontend/admin/bun.lock frontend/admin/bunfig.toml ./
RUN bun install --frozen-lockfile
COPY frontend/admin/ .
RUN bun run build

# --- 编译wiki前端 ---
WORKDIR /app/frontend/wiki
COPY frontend/wiki/package.json frontend/wiki/bun.lock frontend/wiki/bunfig.toml ./
RUN bun install --frozen-lockfile
COPY frontend/wiki/ .
RUN bun run build

# --- Chef准备 ---
FROM rust:alpine AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- 共享构建基础镜像：安装编译依赖和 cargo-chef ---
FROM rust:alpine AS builder-base
RUN cargo install cargo-chef --locked
WORKDIR /app

# --- 编译所有 Rust 二进制 ---
FROM builder-base AS builder
COPY --from=chef /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --workspace && \
    mkdir -p /app/bin && \
    cp /app/target/release/akasha-backend /app/bin/akasha-backend

# --- Akasha ---
FROM alpine:latest AS akasha
LABEL authors="Trrrrw"
WORKDIR /app

COPY assets ./assets
COPY --from=bun-builder /app/frontend/admin/dist /app/frontend/admin/dist
COPY --from=bun-builder /app/frontend/wiki/dist /app/frontend/wiki/dist
COPY --from=builder /app/bin/akasha-backend /app/akasha-backend

RUN chmod +x /app/akasha-backend
CMD ["./akasha-backend"]

# --- worker ---
# --- 安装worker依赖 ---
FROM oven/bun:alpine AS worker-deps
WORKDIR /app/worker
COPY worker/package.json worker/bun.lock worker/bunfig.toml ./
RUN bun install --frozen-lockfile --production
# --- worker runtime ---
FROM oven/bun:alpine AS worker
LABEL authors="Trrrrw"
WORKDIR /app/worker

COPY --from=worker-deps /app/worker/node_modules ./node_modules
COPY worker/package.json ./package.json
COPY worker/src ./src
COPY worker/run.sh ./run.sh

ENV TZ=Asia/Shanghai
RUN apk add --no-cache tzdata

RUN chmod +x run.sh
CMD ["sh", "run.sh"]
