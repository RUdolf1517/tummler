# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.3.2
git push origin v0.3.2
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

v0.3.2 hashes:

```
eea76f9ce9b53106687c007ef61c66a3b6de904ad8cd80a3d0afa66f4e1a39d7  tummler-aarch64-apple-darwin.tar.gz
aba7b09d3fc42286a4edd9201e8052b0256704d0b5b28e482940c780ee746093  tummler-aarch64-unknown-linux-gnu.tar.gz
39648d9d2ea677744483e220f7d362b7ef261d10b9d1aec6a6b88f6647f0e59f  tummler-x86_64-apple-darwin.tar.gz
410a2b65d8c7f2190da432e2df7463e67bdc60ef39e1bca7c85280b11fffc890  tummler-x86_64-unknown-linux-gnu.tar.gz
```

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
