$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path
$probeRoot = Join-Path $PSScriptRoot 'gate-probe'
$originalPath = $env:PATH
$originalFailure = $env:DEVSWEEP_PROBE_FAIL
try {
    $env:PATH = (Join-Path $probeRoot 'mock-bin') + [IO.Path]::PathSeparator + $originalPath
    foreach ($failedStep in @('types:generate', 'lint', 'typecheck', 'test', 'build')) {
        $env:DEVSWEEP_PROBE_FAIL = $failedStep
        & rtk proxy just --justfile (Join-Path $repoRoot 'justfile') --working-directory $probeRoot desktop-web-check
        $recipeExit = $LASTEXITCODE
        Write-Output ('INJECTED=' + $failedStep + ' mock_exit=23 recipe_exit=' + $recipeExit)
    }
} finally {
    $env:PATH = $originalPath
    $env:DEVSWEEP_PROBE_FAIL = $originalFailure
}
