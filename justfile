set dotenv-load := true
set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

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
    docker build --target akasha -t akasha-backend:latest .
    docker build --target worker -t akasha-worker:latest .

[windows]
run-docker: build-docker
    docker compose -f docker-compose.dev.yml up -d

[windows]
check:
    cargo fmt --check
    cargo check
    Push-Location frontend/admin; try { bun run build } finally { Pop-Location }
    Push-Location frontend/wiki; try { bun run build } finally { Pop-Location }
    Push-Location worker; try { bun run check } finally { Pop-Location }

[windows]
clean:
    Remove-Item -LiteralPath dist -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath frontend/admin/dist -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath frontend/wiki/dist -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath worker/dist -Recurse -Force -ErrorAction SilentlyContinue
    cargo clean
