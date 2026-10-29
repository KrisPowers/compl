# Builds a release binary for this machine and copies it into builds/.
#
#   pwsh scripts/build.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $name = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'autocomplete.exe' } else { 'autocomplete' }
    New-Item -ItemType Directory -Force -Path builds | Out-Null
    Copy-Item (Join-Path 'target/release' $name) builds -Force
    Write-Host "Built builds/$name"
} finally {
    Pop-Location
}
