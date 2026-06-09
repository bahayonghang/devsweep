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

dev:
    cargo run -- tui

ci: fmt check test clippy
    @echo "ci complete"
