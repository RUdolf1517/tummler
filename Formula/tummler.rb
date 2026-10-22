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
  version "0.4.0"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.0/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "db6de23b52b7fb78825d23415bf8b4311281408452fc9518f98237e55fbcf182"
    end
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.0/tummler-x86_64-apple-darwin.tar.gz"
      sha256 "ffef520d550b31edb3f799f5e1378859b72b8ada3c322edbd111e5a7f404d53b"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.0/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "19de8e44449b75e3485888d950424b5e5ea4526eb8a7f46822c251a6db64dcb8"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.4.0/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "2a0dfb85e3c57c47902387b68e346338be9272bea36bbf27c493d97e16667243"
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
