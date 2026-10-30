#!/usr/bin/env bash
# Prints the Homebrew formula for a release.
#   bash scripts/formula.sh 0.1.0 dist/checksums.txt > Formula/compl.rb
set -euo pipefail

version="$1"
checksums="$2"
base="https://compl.krispowers.dev/download/v$version"

sha() {
    awk -v file="$1.tar.gz" '$2 == file { print $1 }' "$checksums"
}

for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
    if [ -z "$(sha "$target")" ]; then
        echo "no checksum for $target in $checksums" >&2
        exit 1
    fi
done

cat <<EOF
class Compl < Formula
  desc "Inline, as-you-type suggestions for commands and paths in any terminal"
  homepage "https://github.com/KrisPowers/compl"
  version "$version"
  license "MIT"

  on_macos do
    on_arm do
      url "$base/aarch64-apple-darwin.tar.gz"
      sha256 "$(sha aarch64-apple-darwin)"
    end
    on_intel do
      url "$base/x86_64-apple-darwin.tar.gz"
      sha256 "$(sha x86_64-apple-darwin)"
    end
  end

  on_linux do
    on_arm do
      url "$base/aarch64-unknown-linux-musl.tar.gz"
      sha256 "$(sha aarch64-unknown-linux-musl)"
    end
    on_intel do
      url "$base/x86_64-unknown-linux-musl.tar.gz"
      sha256 "$(sha x86_64-unknown-linux-musl)"
    end
  end

  def install
    bin.install "compl"
  end

  def caveats
    <<~TEXT
      Turn suggestions on for your shell, then open a new terminal:
        compl setup
    TEXT
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/compl --version")
  end
end
EOF
