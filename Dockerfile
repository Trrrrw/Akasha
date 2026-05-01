# --- Akasha 编译 ---
FROM rust:alpine AS akasha-builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin Akasha

# --- 爬虫编译 ---
FROM rust:alpine AS crawler-builder
WORKDIR /app
COPY . .
RUN cargo build --release -p crawler --bin crawler

# --- Akasha ---
FROM alpine:latest AS akasha
LABEL authors="Trrrrw"
WORKDIR /app
COPY --from=akasha-builder /app/target/release/Akasha /app/Akasha
RUN chmod +x /app/Akasha
CMD ["./Akasha"]

# --- 爬虫 ---
FROM alpine:latest AS crawler
LABEL authors="Trrrrw"
WORKDIR /app
COPY --from=crawler-builder /app/target/release/crawler /app/crawler
RUN chmod +x /app/crawler
CMD ["./crawler", "serve"]
