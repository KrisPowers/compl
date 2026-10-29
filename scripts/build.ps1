# Builds a release binary for this machine and copies it into builds/.
#
#   powershell -File scripts/build.ps1

# Not 'Stop': Windows PowerShell turns cargo's progress output on stderr into
# errors once output is redirected. The exit code is checked instead.
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    cargo build --release 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $name = if ($env:OS -eq 'Windows_NT') { 'autocomplete.exe' } else { 'autocomplete' }
    New-Item -ItemType Directory -Force -Path builds | Out-Null
    Copy-Item (Join-Path 'target/release' $name) builds -Force -ErrorAction Stop
    Write-Host "Built builds/$name"
} finally {
    Pop-Location
}
