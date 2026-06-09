set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default: ci

fmt:
    cargo fmt --all -- --check

check:
    cargo check --all-targets

test:
    cargo test --all-targets

clippy:
    cargo clippy --all-targets -- -D warnings

build:
    cargo build

release-archive:
    cargo build --release
    New-Item -ItemType Directory -Force -Path dist | Out-Null
    if (Test-Path 'dist\devsweep-x86_64-pc-windows-msvc.zip') { Remove-Item 'dist\devsweep-x86_64-pc-windows-msvc.zip' }
    Compress-Archive -LiteralPath 'target\release\devsweep.exe' -DestinationPath 'dist\devsweep-x86_64-pc-windows-msvc.zip'
    Write-Output 'release archive: dist\devsweep-x86_64-pc-windows-msvc.zip'

dev:
    cargo run -- tui

ci: fmt check test clippy
    @echo "ci complete"
