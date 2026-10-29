# Installs autocomplete on Windows from the latest GitHub release.
#
#   irm https://raw.githubusercontent.com/KrisPowers/autocomplete/main/scripts/install.ps1 | iex
#
# AUTOCOMPLETE_VERSION      a release tag such as v0.1.0 (default: latest)
# AUTOCOMPLETE_INSTALL_DIR  where the binary goes (default: %LOCALAPPDATA%\Programs\autocomplete)
# AUTOCOMPLETE_NO_SETUP     set to skip adding autocomplete to your PowerShell profile

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'KrisPowers/autocomplete'
    $version = if ($env:AUTOCOMPLETE_VERSION) { $env:AUTOCOMPLETE_VERSION } else { 'latest' }
    $dir = if ($env:AUTOCOMPLETE_INSTALL_DIR) { $env:AUTOCOMPLETE_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\autocomplete' }

    $arm = $env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or $env:PROCESSOR_ARCHITEW6432 -eq 'ARM64'
    if (-not [Environment]::Is64BitOperatingSystem) { throw 'autocomplete needs 64-bit Windows' }
    $target = if ($arm) { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
    $base = if ($version -eq 'latest') {
        "https://github.com/$repo/releases/latest/download"
    } else {
        "https://github.com/$repo/releases/download/$version"
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "Downloading autocomplete for $target"
        $zip = Join-Path $tmp "$target.zip"
        $sums = Join-Path $tmp 'checksums.txt'
        Invoke-WebRequest "$base/$target.zip" -OutFile $zip -UseBasicParsing
        Invoke-WebRequest "$base/checksums.txt" -OutFile $sums -UseBasicParsing

        $line = Get-Content $sums | Where-Object { ($_ -split '\s+')[1] -eq "$target.zip" }
        $expected = if ($line) { ($line -split '\s+')[0] } else { '' }
        $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
        if (-not $expected -or $expected -ne $actual) { throw "checksum mismatch for $target.zip" }

        Expand-Archive $zip -DestinationPath (Join-Path $tmp 'out') -Force
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Copy-Item (Join-Path $tmp 'out\autocomplete.exe') $dir -Force
    } finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
    Write-Host "Installed $dir\autocomplete.exe"

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = if ($userPath) { $userPath -split ';' } else { @() }
    if ($entries -notcontains $dir) {
        $newPath = (@($entries | Where-Object { $_ }) + $dir) -join ';'
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        $env:Path = "$env:Path;$dir"
        Write-Host "Added $dir to your PATH"
    }

    if (-not $env:AUTOCOMPLETE_NO_SETUP) {
        $shell = if ($PSVersionTable.PSEdition -eq 'Core') { 'pwsh' } else { 'powershell' }
        & (Join-Path $dir 'autocomplete.exe') setup $shell
    }
}
