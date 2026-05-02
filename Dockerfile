# --- 编译admin前端 ---
FROM oven/bun:alpine AS frontend-builder
WORKDIR /app/admin
COPY admin/package.json admin/bun.lock ./
RUN bun install
COPY admin/ .
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
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --workspace && \
    mkdir -p /app/bin && \
    cp /app/target/release/Akasha /app/bin/Akasha && \
    cp /app/target/release/crawler /app/bin/crawler

# --- Akasha ---
FROM alpine:latest AS akasha
LABEL authors="Trrrrw"
WORKDIR /app
COPY --from=frontend-builder /app/admin/dist /app/admin/dist
COPY --from=builder /app/bin/Akasha /app/Akasha
RUN chmod +x /app/Akasha
CMD ["./Akasha"]

# --- 爬虫 ---
FROM alpine:latest AS crawler
LABEL authors="Trrrrw"
WORKDIR /app
COPY --from=builder /app/bin/crawler /app/crawler
RUN chmod +x /app/crawler
CMD ["./crawler", "serve"]
