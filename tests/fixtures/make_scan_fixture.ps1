param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$fixtureRoot = [System.IO.Path]::GetFullPath($OutputPath)
$workspaceRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
)
$separator = [System.IO.Path]::DirectorySeparatorChar
$insideWorkspace = $fixtureRoot.StartsWith($workspaceRoot + $separator, [System.StringComparison]::OrdinalIgnoreCase)
$insideTemp = $fixtureRoot.StartsWith($tempRoot + $separator, [System.StringComparison]::OrdinalIgnoreCase)

if (-not $insideWorkspace -and -not $insideTemp) {
    throw "fixture output must be inside the repository or system temp directory: $fixtureRoot"
}
if ((Split-Path -Leaf $fixtureRoot) -ne 'devsweep-core-api-extraction-fixture') {
    throw "fixture output must use the dedicated leaf directory: $fixtureRoot"
}

function Assert-NoReparsePointInPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidatePath,
        [Parameter(Mandatory = $true)]
        [string]$AllowedRoot
    )

    $current = $CandidatePath
    while ($current.Length -gt $AllowedRoot.Length) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "fixture output path must not traverse a reparse point: $current"
            }
        }

        $parent = Split-Path -Parent $current
        if ($parent -eq $current) {
            break
        }
        $current = $parent
    }
}

$allowedRoot = if ($insideWorkspace) { $workspaceRoot } else { $tempRoot }
Assert-NoReparsePointInPath -CandidatePath $fixtureRoot -AllowedRoot $allowedRoot

if (Test-Path -LiteralPath $fixtureRoot) {
    Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
}

$files = [ordered]@{
    'rust-app\Cargo.toml' = (@(
        '[package]'
        'name = "fixture-rust-app"'
        'version = "0.1.0"'
        'edition = "2021"'
        ''
        '[workspace]'
    ) -join "`n") + "`n"
    'rust-app\src\lib.rs' = "pub fn fixture() {}`n"
    'rust-app\target\debug\fixture.bin' = 'rust-target'
    'node-app\package.json' = (@(
        '{'
        '  "name": "fixture-node-app",'
        '  "private": true'
        '}'
    ) -join "`n") + "`n"
    'node-app\node_modules\fixture\index.js' = "module.exports = 'fixture';`n"
    'python-app\pyproject.toml' = (@(
        '[project]'
        'name = "fixture-python-app"'
        'version = "0.1.0"'
    ) -join "`n") + "`n"
    'python-app\.venv\pyvenv.cfg' = "home = fixture-python`n"
    'python-app\package\__pycache__\module.pyc' = 'python-bytecode'
}

foreach ($entry in $files.GetEnumerator()) {
    $path = Join-Path $fixtureRoot $entry.Key
    $parent = Split-Path -Parent $path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    [System.IO.File]::WriteAllText($path, $entry.Value, [System.Text.UTF8Encoding]::new($false))
}

$fixedTimestamp = [DateTime]::SpecifyKind([DateTime]'2024-01-01T00:00:00', [DateTimeKind]::Utc)
Get-ChildItem -LiteralPath $fixtureRoot -Force -Recurse |
    Sort-Object { $_.FullName.Length } -Descending |
    ForEach-Object { $_.LastWriteTimeUtc = $fixedTimestamp }
(Get-Item -LiteralPath $fixtureRoot).LastWriteTimeUtc = $fixedTimestamp

Write-Output $fixtureRoot
