# Builds a release binary for this machine, copies it into builds/ and
# installs it where the installers put it, so every build is the one in use.
#
#   powershell -File scripts/build.ps1

# Not 'Stop': Windows PowerShell turns cargo's progress output on stderr into
# errors once output is redirected. The exit code is checked instead.
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    cargo build --release 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $windows = $env:OS -eq 'Windows_NT'
    $name = if ($windows) { 'autocomplete.exe' } else { 'autocomplete' }
    $built = Join-Path 'target/release' $name
    New-Item -ItemType Directory -Force -Path builds | Out-Null
    Copy-Item $built builds -Force -ErrorAction Stop
    Write-Host "Built builds/$name"

    $dir = if ($windows) { Join-Path $env:LOCALAPPDATA 'Programs\autocomplete' } else { Join-Path $HOME '.local/bin' }
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $installed = Join-Path $dir $name
    # Windows cannot overwrite a running program, but it can rename one, so
    # open sessions keep running the old copy while new ones get this build.
    if ($windows -and (Test-Path $installed)) {
        Remove-Item "$installed.old" -Force -ErrorAction SilentlyContinue
        Rename-Item $installed "$name.old" -Force -ErrorAction Stop
    }
    Copy-Item $built $installed -Force -ErrorAction Stop
    Write-Host "Installed $installed"
} finally {
    Pop-Location
}
