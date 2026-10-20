# brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
# brew install tummler
#
# Prebuilt binaries from GitHub Releases (no Rust). Intel Mac: brew install --HEAD.

class Tummler < Formula
  desc "One CLI that mixes any file into any format"
  homepage "https://github.com/RUdolf1517/tummler"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.1.0/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "6fe6b23d5a6c95e6bf0daf8ef00831a6851541d13395867ea8b3b87865093725"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.1.0/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "561a3bcae0cb3b0c59068f5f01056694f6ab508f833a22bff1eab4f0cfffc296"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.1.0/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "4000f8bf5e5a7c786c5caaeb9785f65826571fd4d17f5f3b75da052ce6adfb45"
    end
  end

  head do
    url "https://github.com/RUdolf1517/tummler.git", branch: "main"
    depends_on "pkg-config" => :build
    depends_on "rust" => :build
    depends_on "xz" => :build
  end

  def install
    if build.head?
      system "cargo", "install", "--root", prefix, "--path", "."
    else
      bin.install "tummler"
    end
  end

  test do
    assert_match "tummler", shell_output("#{bin}/tummler --version")
  end
end
