# Third-party tap (Homebrew 6+ tap trust). Custom remote: this repo is not
# named homebrew-tummler, so brew tap needs the GitHub URL.
#
#   brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
#   brew install RUdolf1517/tummler/tummler
#
# Short name: brew trust --formula RUdolf1517/tummler/tummler && brew install tummler
# Intel Mac:  brew install --HEAD RUdolf1517/tummler/tummler

class Tummler < Formula
  desc "One CLI that mixes any file into any format"
  homepage "https://github.com/RUdolf1517/tummler"
  version "0.3.0"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.0/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "12d8329289a716fbd32d003ca33e8d6e0c6b292dd507ca7e6e4fbaf2fc66266e"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.0/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "93e4e7fa6949d37c44a4868f3e2374b2b90d4118cf0764e694adff9951f406da"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.0/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "d53a12011fa5c1c4778c825d82e4c0d07779b736f7bf1a6e18663b4737e9e084"
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
    (man1/"tummler.1").write Utils.safe_popen_read(bin/"tummler", "--man")
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

        No Intel Mac bottle. Use: brew install --HEAD tummler
      EOS
    end
    s
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tummler --version")
    assert_match "png", shell_output("#{bin}/tummler --list-formats")
    assert_match ".TH TUMMLER", shell_output("#{bin}/tummler --man")
  end
end
