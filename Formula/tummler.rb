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
  version "0.5.0"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.5.0/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "d39ab07ac44424975c334f5ac2bd01b3b82b99a7c23aa1f5a2df04662aef26e4"
    end
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.5.0/tummler-x86_64-apple-darwin.tar.gz"
      sha256 "c9243d7d835046bd896c008b76a6ada0609921dc9ff7513611422a03c7557388"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.5.0/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0475ca8c570dd35c1058d9b40d651dd1047bd70f56da8d6dad709468da1c02de"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.5.0/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "60cc6788fd58aa76942537cce2c25d4762cb500303a8b55ad712e871f25cf67c"
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
