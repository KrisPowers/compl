# Installation options

The quick installs are in the [README](../README.md#installation). This page covers the details.

## Installer variables

Both installers read these environment variables:

| Variable | Default | Effect |
| --- | --- | --- |
| `AUTOCOMPLETE_VERSION` | `latest` | Install a specific release tag, such as `v0.1.0` |
| `AUTOCOMPLETE_INSTALL_DIR` | `~/.local/bin`, or `%LOCALAPPDATA%\Programs\autocomplete` on Windows | Where the binary goes |
| `AUTOCOMPLETE_NO_SETUP` | unset | Set it to skip `autocomplete setup` |

For example:

```bash
curl -fsSL https://raw.githubusercontent.com/KrisPowers/autocomplete/main/scripts/install.sh | AUTOCOMPLETE_VERSION=v0.1.0 sh
```

```powershell
$env:AUTOCOMPLETE_NO_SETUP = 1; irm https://raw.githubusercontent.com/KrisPowers/autocomplete/main/scripts/install.ps1 | iex
```

Both installers check the download against the release's `checksums.txt` before installing it.

## Release archives

Each [release](https://github.com/KrisPowers/autocomplete/releases) has one archive per platform, named after its Rust target:

| Platform | Archive |
| --- | --- |
| macOS, Apple silicon | `aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `x86_64-apple-darwin.tar.gz` |
| Linux, x86_64 | `x86_64-unknown-linux-musl.tar.gz` |
| Linux, ARM64 | `aarch64-unknown-linux-musl.tar.gz` |
| Windows, x64 | `x86_64-pc-windows-msvc.zip` |
| Windows, ARM64 | `aarch64-pc-windows-msvc.zip` |

The Linux builds are static, so they run on any distribution. To install by hand, extract the binary somewhere on your PATH and run `autocomplete setup`.

## Turning it on for more shells

`autocomplete setup` picks your current shell. Name another one to set it up too:

```bash
autocomplete setup fish
```

## Uninstalling

Remove the startup line, then the binary:

```bash
autocomplete setup --remove
rm ~/.local/bin/autocomplete
```

With Homebrew, run `autocomplete setup --remove` and then `brew uninstall autocomplete`. On Windows, run `autocomplete setup powershell --remove` (or `pwsh`) and delete `%LOCALAPPDATA%\Programs\autocomplete`.
