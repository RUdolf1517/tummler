# brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
# brew install --HEAD tummler
#
# After tag v0.1.0, replace `head` with binary `url`/`sha256` per OS
# (see RELEASE.md). Until then this builds from git (needs Rust).

class Tummler < Formula
  desc "One CLI that mixes any file into any format"
  homepage "https://github.com/RUdolf1517/tummler"
  license "MIT"
  head "https://github.com/RUdolf1517/tummler.git", branch: "main"

  depends_on "pkg-config" => :build
  depends_on "rust" => :build
  depends_on "xz" => :build

  def install
    system "cargo", "install", "--root", prefix, "--path", "."
  end

  test do
    assert_match "tummler", shell_output("#{bin}/tummler --version")
  end
end
