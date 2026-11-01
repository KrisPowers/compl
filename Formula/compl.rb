class Compl < Formula
  desc "Inline, as-you-type suggestions for commands and paths in any terminal"
  homepage "https://github.com/KrisPowers/compl"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://compl.krispowers.dev/download/v1.0.0/aarch64-apple-darwin.tar.gz"
      sha256 "c8aef8ba9bceb093783ffab57b89f22cdb9f08fda671d0a83dab6d428dad5976"
    end
    on_intel do
      url "https://compl.krispowers.dev/download/v1.0.0/x86_64-apple-darwin.tar.gz"
      sha256 "b8811f49634d128b53944120e9fee18da89e289bae2e96efda483a57dadb4911"
    end
  end

  on_linux do
    on_arm do
      url "https://compl.krispowers.dev/download/v1.0.0/aarch64-unknown-linux-musl.tar.gz"
      sha256 "4af66896b7e77d673c252b09c45441a9745d416f115de08f48033d684dbff3e2"
    end
    on_intel do
      url "https://compl.krispowers.dev/download/v1.0.0/x86_64-unknown-linux-musl.tar.gz"
      sha256 "7d2a20642987ad7b7fdc74b9ed53ab299b165aa846aeaab7d6e08a75a96415dc"
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
