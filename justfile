set dotenv-load

backend:
    clear
    cargo run -p Akasha

crawler +args:
    clear
    cargo run -p crawler -- {{ args }}

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
    cd admin && bun run build
    cargo build --release --workspace

build-docker:
    clear
    docker build --target akasha -t akasha:latest .
    docker build --target crawler -t akasha-crawler:latest .

check:
    cargo fmt
    cargo check

clean:
    clear
    rm -rf admin/dist
    cargo clean
