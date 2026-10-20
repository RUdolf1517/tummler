# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes, then crates.io if you want it.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.1.1
git push origin v0.1.1
```

Actions builds Apple Silicon, Linux x86_64, and Linux ARM tarballs and attaches them to the release. Intel Mac is not bottled (`brew install --HEAD`).

Then this works:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

## 2. Homebrew

Copy SHA-256 from the release notes into `Formula/tummler.rb` (`url`/`sha256` per OS), bump `version`, commit to `main`. Then:

```bash
brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
brew install tummler
```

v0.1.0 hashes:

```
6fe6b23d5a6c95e6bf0daf8ef00831a6851541d13395867ea8b3b87865093725  tummler-aarch64-apple-darwin.tar.gz
4000f8bf5e5a7c786c5caaeb9785f65826571fd4d17f5f3b75da052ce6adfb45  tummler-aarch64-unknown-linux-gnu.tar.gz
561a3bcae0cb3b0c59068f5f01056694f6ab508f833a22bff1eab4f0cfffc296  tummler-x86_64-unknown-linux-gnu.tar.gz
```

## 3. crates.io (optional)

Name `tummler` is free. One-time: [crates.io/settings/tokens](https://crates.io/settings/tokens) → `cargo login`.

```bash
cargo publish
```

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
