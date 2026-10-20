# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes, then crates.io if you want it.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.2.1
git push origin v0.2.1
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

v0.2.0 hashes:

```
5f10303422666909898d55e6f13c90254ae84a36e6259bb83c223c86ca6fa076  tummler-aarch64-apple-darwin.tar.gz
f0671575ce351f836c40a82d6cedfcf268be90b263de6f0db2a253b51cd01447  tummler-aarch64-unknown-linux-gnu.tar.gz
a21aa4cc19d596ff623894098031927ea4972d508ac8afe74bcc3955b79ff888  tummler-x86_64-unknown-linux-gnu.tar.gz
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
