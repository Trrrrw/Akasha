set dotenv-load := true

backend:
    clear
    cargo run -p Akasha

crawler +args:
    clear
    cargo run -p crawler -- {{ args }}

admin:
    cd admin && bun run dev

dev:
    clear
    bash -c '\
      trap "kill 0" INT TERM EXIT; \
      cargo run -p Akasha & \
      cargo run -p crawler -- serve & \
      (cd admin && bun run dev) & \
      wait \
    '

build:
    clear
    # cd admin && bun run build
    cargo build --release --workspace

build-docker:
    clear
    docker build --target akasha -t akasha-backend:latest .
    docker build --target worker -t akasha-worker:latest .

run-docker: build-docker
    clear
    docker compose -f docker-compose.dev.yml up -d

check:
    cargo fmt
    cargo check
    cd admin && bun run build

clean:
    clear
    rm -rf admin/dist
    cargo clean
