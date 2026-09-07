set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default: ci

help:
    @just --list

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

# Install the CLI into Cargo's bin directory
install:
    cargo install --locked --path crates/devsweep-cli --bin devsweep

# Copy skill packages from skills/ into local agent skill directories
[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
install-skill:
    $ErrorActionPreference = 'Stop'
    $srcRoot = Join-Path (Get-Location) 'skills'
    if (-not (Test-Path -LiteralPath $srcRoot)) {
        throw "skill source directory missing: $srcRoot"
    }
    $packages = @(Get-ChildItem -LiteralPath $srcRoot -Directory |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'SKILL.md') })
    if ($packages.Count -eq 0) {
        throw 'no skill packages with SKILL.md under skills/'
    }
    $destRoots = @(
        (Join-Path (Get-Location) '.agents\skills'),
        (Join-Path (Get-Location) '.claude\skills')
    )
    foreach ($destRoot in $destRoots) {
        New-Item -ItemType Directory -Force -Path $destRoot | Out-Null
        foreach ($pkg in $packages) {
            $dest = Join-Path $destRoot $pkg.Name
            if (Test-Path -LiteralPath $dest) {
                Remove-Item -LiteralPath $dest -Recurse -Force
            }
            Copy-Item -LiteralPath $pkg.FullName -Destination $dest -Recurse
            if (-not (Test-Path -LiteralPath (Join-Path $dest 'SKILL.md'))) {
                throw "skill install missing SKILL.md: $dest"
            }
            Write-Output ('installed skill: ' + $dest)
        }
    }

# Install CLI, desktop app, and local agent skills
install-all: install tinstall install-skill
    @echo "install-all complete"

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
    $tripleLine = rustc -vV | Select-String 'host:'
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $triple = $tripleLine.ToString().Split(' ')[1]
    $zip = Join-Path 'dist' ("devsweep-" + $triple + '.zip')
    $sidecar = $zip + '.sha256'
    if (-not (Test-Path -LiteralPath $zip)) { throw "missing archive $zip; run just release-archive first" }
    if (-not (Test-Path -LiteralPath $sidecar)) { throw "missing sidecar $sidecar; run just release-archive first" }
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
    $zipHash = ([System.BitConverter]::ToString($hashBytes)).Replace('-', '').ToLowerInvariant()
    $sidecarLine = (Get-Content -LiteralPath $sidecar -Raw).Trim()
    if ($sidecarLine -notmatch '^([0-9a-fA-F]{64})\s+\S+') {
        throw ("invalid sha256 sidecar: " + $sidecar)
    }
    $sidecarHash = $Matches[1].ToLowerInvariant()
    if ($zipHash -ne $sidecarHash) {
        throw ("archive hash mismatch for " + $zip + ": file=" + $zipHash + " sidecar=" + $sidecarHash)
    }
    $zipPath = (Resolve-Path -LiteralPath $zip).Path
    Write-Output ("release archive: " + $zipPath)
    Write-Output ("sidecar sha256: " + $sidecarHash)
    $tmp = Join-Path $env:TEMP ("devsweep-smoke-" + [guid]::NewGuid().ToString())
    try {
        New-Item -ItemType Directory -Force -Path $tmp | Out-Null
        $extract = Join-Path $tmp 'archive'
        $fixture = Join-Path $tmp 'fixture'
        $work = Join-Path $tmp 'work'
        $isolatedHome = Join-Path $tmp 'home'
        New-Item -ItemType Directory -Force -Path $extract, $fixture, $work, $isolatedHome | Out-Null
        Expand-Archive -LiteralPath $zip -DestinationPath $extract
        $bin = Join-Path $extract 'devsweep.exe'
        if (-not (Test-Path -LiteralPath $bin)) {
            throw ("release archive contains no devsweep.exe: " + $zip)
        }
        if ($bin -match '[\\/]target[\\/]release[\\/]') {
            throw 'release-smoke must use the archived binary, not target/release'
        }
        $env:LOCALAPPDATA = $isolatedHome
        $env:APPDATA = $isolatedHome
        $env:XDG_CONFIG_HOME = $isolatedHome
        $env:HOME = $isolatedHome
        & $bin --version
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Set-Content -LiteralPath (Join-Path $fixture 'package.json') -Value '{}' -Encoding ascii
        New-Item -ItemType Directory -Force -Path (Join-Path $fixture 'node_modules\pkg') | Out-Null
        Set-Content -LiteralPath (Join-Path $fixture 'node_modules\pkg\index.js') -Value 'module.exports = {}' -Encoding ascii
        $observation = Join-Path $work 'observation.json'
        $plan = Join-Path $work 'plan.json'
        & $bin clean scan --root $fixture --scope projects --format json --output $observation
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        $document = Get-Content -LiteralPath $observation -Raw | ConvertFrom-Json
        $targets = @()
        if ($null -ne $document.data -and $null -ne $document.data.plan) {
            $targets = @($document.data.plan.targets)
        }
        if ($targets.Count -lt 1) {
            throw 'release-smoke observation contained no cleanup targets'
        }
        $targetId = [string]$targets[0].id
        if ([string]::IsNullOrWhiteSpace($targetId)) {
            throw 'release-smoke observation target id was empty'
        }
        Write-Output ("selected target: " + $targetId)
        & $bin clean plan --observation $observation --select $targetId --output $plan
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        & $bin clean preview --plan $plan --format json
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Write-Output 'release smoke ok'
    }
    finally {
        if ($tmp -and (Test-Path -LiteralPath $tmp)) {
            Remove-Item -LiteralPath $tmp -Recurse -Force
        }
    }

dev:
    cargo run --locked -p devsweep-cli --bin devsweep

docs:
    npm run docs:dev

[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
desktop-web-check:
    $ErrorActionPreference = 'Stop'
    Set-Location -LiteralPath (Join-Path (Get-Location) 'desktop')
    npm run types:generate -- --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    npm run lint
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    npm run typecheck
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    npm test
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    npm run build
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

desktop-test:
    cargo test --locked -p devsweep-desktop

desktop-build:
    cd desktop; npm run tauri -- build

# Build the unsigned NSIS installer and silently install the Tauri desktop app
[script("powershell.exe", "-NoLogo", "-NoProfile", "-File")]
tinstall: desktop-build
    $ErrorActionPreference = 'Stop'
    $nsisDirs = @(
        'target\release\bundle\nsis',
        'desktop\src-tauri\target\release\bundle\nsis'
    )
    $setup = $nsisDirs |
        Where-Object { Test-Path $_ } |
        ForEach-Object { Get-ChildItem -Path $_ -Filter '*-setup.exe' } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($null -eq $setup) {
        throw 'NSIS setup exe missing; expected target\release\bundle\nsis\*-setup.exe'
    }
    Write-Output ('installing: ' + $setup.FullName)
    $proc = Start-Process -FilePath $setup.FullName -ArgumentList '/S' -Wait -PassThru
    if ($null -eq $proc) { throw 'NSIS installer did not start' }
    if ($proc.ExitCode -ne 0) {
        throw ('NSIS installer failed with exit code ' + $proc.ExitCode)
    }
    Write-Output 'desktop install complete'

ci: fmt check test clippy
    @echo "ci complete"
