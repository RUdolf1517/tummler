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
  version "0.3.3"
  license "MIT"

  livecheck do
    url :homepage
    strategy :github_latest
  end

  on_macos do
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.3/tummler-aarch64-apple-darwin.tar.gz"
      sha256 "04ec179a2bac503c1fc9c3c04ba0e856da31a98d1750dcc3258d03741b6f88f0"
    end
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.3/tummler-x86_64-apple-darwin.tar.gz"
      sha256 "cae62cfe04c8117bc7a0a73e292c0ae0fc19f596f8351635313b33883bdf7522"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.3/tummler-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "956170992a60bc3cb73dc3255da18149a12905d8f3b457e0395b0ae99fe0e6b5"
    end
    on_arm do
      url "https://github.com/RUdolf1517/tummler/releases/download/v0.3.3/tummler-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "8465eded9d2fdd6f8da468e09fdb4aad3284c78feecde7e9f30f5ec0f9366477"
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
