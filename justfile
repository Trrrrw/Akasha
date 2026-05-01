set dotenv-load

alias d := dev
dev:
    clear
    bash -c '\
      trap "kill 0" INT TERM EXIT; \
      cargo run -p Akasha & \
      cargo run -p crawler -- serve & \
      wait \
    '

backend:
    clear
    cargo run -p Akasha

crawler:
    clear
    cargo run -p crawler -- serve

check:
    cargo fmt
    cargo check

alias b := build
build:
    clear
    cargo build --release --workspace

alias bd := build-docker
build-docker:
    clear
    docker build --target akasha -t akasha:latest .
    docker build --target crawler -t akasha-crawler:latest .

clean:
    clear
    cargo clean
