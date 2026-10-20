# Cut a release

Do this **after** merging to `main`. Order matters: binaries first, then crates.io, then Homebrew.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.1.0
git push origin v0.1.0
```

Actions builds macOS/Linux x86_64+ARM tarballs and attaches them to the release.

Then this works:

```bash
curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
```

## 2. crates.io

Name `tummler` is free. One-time: [crates.io/settings/tokens](https://crates.io/settings/tokens) → `cargo login`.

```bash
cargo publish
```

Then:

```bash
cargo install tummler
```

## 3. Homebrew

Until v0.1.0 the formula is `--HEAD` (builds with Rust):

```bash
brew tap RUdolf1517/tummler https://github.com/RUdolf1517/tummler
brew install --HEAD tummler
```

After the GitHub Release, fill `Formula/tummler.rb` with the four `url`/`sha256` pairs:

```bash
curl -fsSL -o /tmp/a.tgz https://github.com/RUdolf1517/tummler/releases/download/v0.1.0/tummler-aarch64-apple-darwin.tar.gz
shasum -a 256 /tmp/a.tgz
# repeat for x86_64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
```

Then `brew install tummler` (no Rust).

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
