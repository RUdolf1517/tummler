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
  version "0.3.2"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.2/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "eea76f9ce9b53106687c007ef61c66a3b6de904ad8cd80a3d0afa66f4e1a39d7"
    end
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.2/tummler-x86_64-apple-darwin.tar.gz"
      sha256 "39648d9d2ea677744483e220f7d362b7ef261d10b9d1aec6a6b88f6647f0e59f"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.2/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "410a2b65d8c7f2190da432e2df7463e67bdc60ef39e1bca7c85280b11fffc890"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.2/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "aba7b09d3fc42286a4edd9201e8052b0256704d0b5b28e482940c780ee746093"
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
