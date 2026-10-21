# Third-party tap (Homebrew 6+ tap trust). Custom remote: this repo is not
# named homebrew-tummler, so brew tap needs the GitHub URL.
#
#   brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
#   brew install RUdolf1517/tummler/tummler
#
# Short name: brew trust --formula RUdolf1517/tummler/tummler && brew install tummler
# Intel Mac bottle: tummler-x86_64-apple-darwin.tar.gz (url/sha256 after the next tag).
# Until then: brew install --HEAD RUdolf1517/tummler/tummler

class Tummler < Formula
  desc "One CLI that mixes any file into any format"
  homepage "https://github.com/RUdolf1517/tummler"
  version "0.3.1"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.1/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "6a8bc2079b7cf50726ec4bdfc7c4bace26ab8e3dac0c6e02b44b927585006d4a"
    end
    # on_intel: add url/sha256 for tummler-x86_64-apple-darwin.tar.gz after the next tag
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.1/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9dfe8fda4ed4ed63b9a2067b1b5ce56240d805a11faccce07ac21c9c806a8576"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.1/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f6755647926f7dc2206d4a6e220bc0e46224f53aaccd280fd9c3ea8a54a3329d"
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

    generate_completions_from_executable(bin/"tummler", "--completions",
                                         shells: [:bash, :zsh, :fish])

    # 0.3.0 bottles have --completions but not --man.
    manpage = Utils.popen_read(bin/"tummler", "--man")
    if $CHILD_STATUS.success? && manpage.include?(".TH TUMMLER")
      (man1/"tummler.1").write manpage
    end
  end

  def caveats
    s = <<~EOS
      Optional tools (tummler offers to install them in a TTY):
        ffmpeg   video / HEIC / AVIF
        pandoc   documents
        poppler  PDF → png / docx
      LibreOffice (soffice) is never auto-installed; slides need it already on PATH.
    EOS
    if OS.mac? && Hardware::CPU.intel?
      s += <<~EOS

        No Intel Mac bottle in 0.3.1. Next release ships x86_64-apple-darwin.
        Until the Formula has that sha256: brew install --HEAD RUdolf1517/tummler/tummler
      EOS
    end
    s
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tummler --version")
    assert_match "png", shell_output("#{bin}/tummler --list-formats")
    if shell_output("#{bin}/tummler --help").include?("--man")
      assert_match ".TH TUMMLER", shell_output("#{bin}/tummler --man")
    end
  end
end
