set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default: ci

fmt:
    cargo fmt --all -- --check

check:
    cargo check --locked --all-targets

test:
    cargo test --locked --all-targets

clippy:
    cargo clippy --locked --all-targets -- -D warnings

build:
    cargo build --locked

release-archive:
    cargo build --locked --release
    New-Item -ItemType Directory -Force -Path dist | Out-Null
    $triple = (rustc -vV | Select-String 'host:').ToString().Split(' ')[1]
    $exe = if (Test-Path 'target\release\devsweep.exe') { 'target\release\devsweep.exe' } else { 'target\release\devsweep' }
    if (-not (Test-Path $exe)) { throw "release binary missing: $exe" }
    $staging = Join-Path 'dist' ("devsweep-" + $triple)
    if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
    New-Item -ItemType Directory -Force -Path $staging | Out-Null
    Copy-Item -LiteralPath $exe -Destination (Join-Path $staging (Split-Path $exe -Leaf))
    Copy-Item -LiteralPath 'LICENSE' -Destination (Join-Path $staging 'LICENSE')
    Copy-Item -LiteralPath 'README.md' -Destination (Join-Path $staging 'README.md')
    $zip = Join-Path 'dist' ("devsweep-" + $triple + '.zip')
    if (Test-Path $zip) { Remove-Item -Force $zip }
    Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zip
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $zip).Hash.ToLowerInvariant()
    Set-Content -LiteralPath ($zip + '.sha256') -Value ($hash + '  ' + (Split-Path $zip -Leaf))
    Write-Output ("release archive: " + $zip)
    Write-Output ("sha256: " + $hash)

release-smoke:
    $triple = (rustc -vV | Select-String 'host:').ToString().Split(' ')[1]
    $zip = Join-Path 'dist' ("devsweep-" + $triple + '.zip')
    if (-not (Test-Path $zip)) { throw "missing archive $zip; run just release-archive first" }
    $tmp = Join-Path $env:TEMP ("devsweep-smoke-" + [guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    Expand-Archive -LiteralPath $zip -DestinationPath $tmp
    $bin = Get-ChildItem -Path $tmp -Recurse -Filter 'devsweep*' | Where-Object { -not $_.PSIsContainer } | Select-Object -First 1
    & $bin.FullName --version
    & $bin.FullName scan --json | Out-Null
    & $bin.FullName clean --plan (Join-Path $tmp 'empty-plan.json') 2>$null; if (-not $?) { '{"version":2,"targets":[]}' | Set-Content (Join-Path $tmp 'empty-plan.json'); & $bin.FullName clean --plan (Join-Path $tmp 'empty-plan.json') }
    Write-Output 'release smoke ok'

dev:
    cargo run --locked -- tui

ci: fmt check test clippy
    @echo "ci complete"
