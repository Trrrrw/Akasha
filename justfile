set windows-shell := ["powershell.exe", "-c"]

[windows]
dev:
    clear
    Remove-Item -LiteralPath './data' -Recurse -Force -Confirm:$false
    cargo run

[unix]
dev:
    clear
    cargo run

check:
    cargo fmt
    cargo check

build:
    clear
    cargo build --release --workspace

clean:
    clear
    cargo clean
