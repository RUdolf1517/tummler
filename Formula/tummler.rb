# brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
# brew install tummler
#
# Prebuilt binaries from GitHub Releases (no Rust). Intel Mac: brew install --HEAD.

class Tummler < Formula
  desc "One CLI that mixes any file into any format"
  homepage "https://github.com/RUdolf1517/tummler"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.2.0/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "5f10303422666909898d55e6f13c90254ae84a36e6259bb83c223c86ca6fa076"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.2.0/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a21aa4cc19d596ff623894098031927ea4972d508ac8afe74bcc3955b79ff888"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.2.0/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f0671575ce351f836c40a82d6cedfcf268be90b263de6f0db2a253b51cd01447"
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
