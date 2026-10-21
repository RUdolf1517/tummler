# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.3.1
git push origin v0.3.1
```

Actions builds Apple Silicon, Intel Mac (`x86_64-apple-darwin`), Linux x86_64, and Linux ARM tarballs and attaches them to the release. After the tag, copy the Intel Mac SHA-256 into `Formula/tummler.rb` (`on_macos` / `on_intel`).

Then this works:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

## 2. Homebrew

Copy SHA-256 from the release notes into `Formula/tummler.rb` (`url`/`sha256` per OS), bump `version`, commit to `main`. Then:

```bash
brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
brew install RUdolf1517/tummler/tummler
```

Homebrew 6+ [tap trust](https://docs.brew.sh/Tap-Trust): the fully qualified name trusts only this formula. Short name needs `brew trust --formula RUdolf1517/tummler/tummler` first. Custom remote is required because this repo is not named `homebrew-tummler`.

v0.3.1 hashes:

```
6a8bc2079b7cf50726ec4bdfc7c4bace26ab8e3dac0c6e02b44b927585006d4a  tummler-aarch64-apple-darwin.tar.gz
f6755647926f7dc2206d4a6e220bc0e46224f53aaccd280fd9c3ea8a54a3329d  tummler-aarch64-unknown-linux-gnu.tar.gz
9dfe8fda4ed4ed63b9a2067b1b5ce56240d805a11faccce07ac21c9c806a8576  tummler-x86_64-unknown-linux-gnu.tar.gz
```

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
