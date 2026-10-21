# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes, then crates.io if you want it.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.3.1
git push origin v0.3.1
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

v0.3.0 hashes:

```
12d8329289a716fbd32d003ca33e8d6e0c6b292dd507ca7e6e4fbaf2fc66266e  tummler-aarch64-apple-darwin.tar.gz
d53a12011fa5c1c4778c825d82e4c0d07779b736f7bf1a6e18663b4737e9e084  tummler-aarch64-unknown-linux-gnu.tar.gz
93e4e7fa6949d37c44a4868f3e2374b2b90d4118cf0764e694adff9951f406da  tummler-x86_64-unknown-linux-gnu.tar.gz
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
