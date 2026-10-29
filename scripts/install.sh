#!/bin/sh
# Installs autocomplete on macOS and Linux from the latest GitHub release.
#
#   curl -fsSL https://raw.githubusercontent.com/KrisPowers/autocomplete/main/scripts/install.sh | sh
#
# AUTOCOMPLETE_VERSION      a release tag such as v0.1.0 (default: latest)
# AUTOCOMPLETE_INSTALL_DIR  where the binary goes (default: ~/.local/bin)
# AUTOCOMPLETE_NO_SETUP     set to skip adding autocomplete to your shell
set -eu

repo="KrisPowers/autocomplete"
version="${AUTOCOMPLETE_VERSION:-latest}"
dir="${AUTOCOMPLETE_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
    echo "autocomplete: $*" >&2
    exit 1
}

case "$(uname -s)" in
    Linux) os=unknown-linux-musl ;;
    Darwin) os=apple-darwin ;;
    *) fail "unsupported system $(uname -s); on Windows run scripts/install.ps1" ;;
esac
case "$(uname -m)" in
    x86_64 | amd64) arch=x86_64 ;;
    arm64 | aarch64) arch=aarch64 ;;
    *) fail "unsupported CPU $(uname -m)" ;;
esac
target="$arch-$os"

if [ "$version" = latest ]; then
    base="https://github.com/$repo/releases/latest/download"
else
    base="https://github.com/$repo/releases/download/$version"
fi

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        fail "curl or wget is required"
    fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading autocomplete for $target"
download "$base/$target.tar.gz" "$tmp/$target.tar.gz"
download "$base/checksums.txt" "$tmp/checksums.txt"

expected="$(awk -v file="$target.tar.gz" '$2 == file { print $1 }' "$tmp/checksums.txt")"
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$target.tar.gz" | awk '{ print $1 }')"
else
    actual="$(shasum -a 256 "$tmp/$target.tar.gz" | awk '{ print $1 }')"
fi
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
    fail "checksum mismatch for $target.tar.gz"
fi

tar -xzf "$tmp/$target.tar.gz" -C "$tmp"
mkdir -p "$dir"
cp "$tmp/autocomplete" "$dir/autocomplete"
chmod 755 "$dir/autocomplete"
echo "Installed $dir/autocomplete"

case ":$PATH:" in
    *":$dir:"*) ;;
    *) echo "Note: $dir is not on your PATH. Add it to run autocomplete by name." ;;
esac

if [ -z "${AUTOCOMPLETE_NO_SETUP:-}" ]; then
    "$dir/autocomplete" setup
fi
