set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default: ci

fmt:
    cargo fmt --all -- --check

sync-lock:
    cargo update --offline --package devsweep-core --package devsweep-cli

check:
    cargo check --workspace --locked --all-targets

test:
    cargo test --workspace --locked --all-targets

clippy:
    cargo clippy --workspace --locked --all-targets -- -D warnings

build:
    cargo build --locked -p devsweep-cli --bin devsweep

[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
release-archive:
    $ErrorActionPreference = 'Stop'
    cargo build --locked --release -p devsweep-cli --bin devsweep
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
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
    $stream = [System.IO.File]::OpenRead($zip)
    try {
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            $hashBytes = $sha256.ComputeHash($stream)
        } finally {
            $sha256.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
    $hash = ([System.BitConverter]::ToString($hashBytes)).Replace('-', '').ToLowerInvariant()
    Set-Content -LiteralPath ($zip + '.sha256') -Value ($hash + '  ' + (Split-Path $zip -Leaf))
    Write-Output ("release archive: " + $zip)
    Write-Output ("sha256: " + $hash)

[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
release-smoke:
    $ErrorActionPreference = 'Stop'
    $triple = (rustc -vV | Select-String 'host:').ToString().Split(' ')[1]
    $zip = Join-Path 'dist' ("devsweep-" + $triple + '.zip')
    if (-not (Test-Path $zip)) { throw "missing archive $zip; run just release-archive first" }
    $tmp = Join-Path $env:TEMP ("devsweep-smoke-" + [guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    Expand-Archive -LiteralPath $zip -DestinationPath $tmp
    $bin = Get-ChildItem -Path $tmp -Recurse -Filter 'devsweep*' | Where-Object { -not $_.PSIsContainer } | Select-Object -First 1
    if ($null -eq $bin) { throw "release archive contains no devsweep binary: $zip" }
    & $bin.FullName --version
    if ($LASTEXITCODE -ne 0) { throw 'release --version smoke failed' }
    & $bin.FullName scan --json | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'release scan smoke failed' }
    $emptyPlan = Join-Path $tmp 'empty-plan.json'
    '{"version":2,"targets":[]}' | Set-Content -LiteralPath $emptyPlan
    & $bin.FullName clean --plan $emptyPlan
    if ($LASTEXITCODE -ne 0) { throw 'release clean dry-run smoke failed' }
    Write-Output 'release smoke ok'

dev:
    cargo run --locked -p devsweep-cli --bin devsweep -- tui

docs:
    npm run docs:dev

ci: fmt sync-lock check test clippy
    @echo "ci complete"
