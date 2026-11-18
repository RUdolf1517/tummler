# Third-party tap (Homebrew 6+ tap trust). Custom remote: this repo is not
# named homebrew-tummler, so brew tap needs the GitHub URL.
#
#   brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
#   brew install RUdolf1517/tummler/tummler
#
# Short name: brew trust --formula RUdolf1517/tummler/tummler && brew install tummler

class Tummler < Formula
  desc "One CLI that mixes any file into any format"
  homepage "https://github.com/RUdolf1517/tummler"
  version "0.4.1"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.1/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "006d4fcb0a9be8a603a7f6c8383690e33d52ae8b51b072f6cb82361404330c16"
    end
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.1/tummler-x86_64-apple-darwin.tar.gz"
      sha256 "d96eea06a8d3ae389ba476e4da18b8bfe5bd109e5b1ea2878dfc53fb27dfed50"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.1/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "83375997b690bd360b7ee53d64b1bb91115338af66865b987073a7a7060d6d8f"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.1/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "502f818abfd6bc8c8286f6746b9739d8b99bf5c354ec358346a6c5a10cb2025a"
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
    <<~EOS
      Optional tools (`tummler --check-deps` offers to install them in a TTY):
        ffmpeg   video / HEIC / AVIF
        pandoc   documents
        poppler  PDF → png / docx
      LibreOffice (soffice) is never auto-installed; slides need it already on PATH.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tummler --version")
    assert_match "png", shell_output("#{bin}/tummler --list-formats")
    if shell_output("#{bin}/tummler --help").include?("--man")
      assert_match ".TH TUMMLER", shell_output("#{bin}/tummler --man")
    end
  end
end
