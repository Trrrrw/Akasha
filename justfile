set dotenv-load
set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

[windows]
restore-db backup container:
    pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts\windows\Restore-1PanelPostgres.ps1 "{{ backup }}" "{{ container }}"

[windows]
db-up:
    pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/windows/db-up.ps1

[windows]
backend: db-up
    pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/windows/start-dev.ps1

[windows]
worker:
    pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/windows/run-worker.ps1

[windows]
build:
    pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/windows/build-package.ps1

[windows]
build-docker:
    docker build -f Dockerfile.dev --target akasha -t akasha-backend:dev .
    docker build -f Dockerfile.dev --target worker -t akasha-worker:dev .

[windows]
run-docker: build-docker
    docker compose up -d

[windows]
check:
    cargo fmt --check
    cargo check
    Push-Location worker; try { bun run check } finally { Pop-Location }

[windows]
clean:
    Remove-Item -LiteralPath dist -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath worker/dist -Recurse -Force -ErrorAction SilentlyContinue
    cargo clean
