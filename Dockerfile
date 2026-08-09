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

# --- 编译后端 ---
FROM builder-base AS builder
COPY --from=chef /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release -p akasha-backend && \
    mkdir -p /app/bin && \
    cp /app/target/release/akasha-backend /app/bin/akasha-backend

# --- Akasha ---
FROM alpine:latest AS akasha
LABEL authors="Trrrrw"
WORKDIR /app

COPY assets ./assets
COPY --from=builder /app/bin/akasha-backend /app/akasha-backend

RUN chmod +x /app/akasha-backend
CMD ["./akasha-backend"]
