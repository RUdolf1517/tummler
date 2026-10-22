# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.3.4
git push origin v0.3.4
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

v0.3.3 hashes:

```
04ec179a2bac503c1fc9c3c04ba0e856da31a98d1750dcc3258d03741b6f88f0  tummler-aarch64-apple-darwin.tar.gz
8465eded9d2fdd6f8da468e09fdb4aad3284c78feecde7e9f30f5ec0f9366477  tummler-aarch64-unknown-linux-gnu.tar.gz
cae62cfe04c8117bc7a0a73e292c0ae0fc19f596f8351635313b33883bdf7522  tummler-x86_64-apple-darwin.tar.gz
956170992a60bc3cb73dc3255da18149a12905d8f3b457e0395b0ae99fe0e6b5  tummler-x86_64-unknown-linux-gnu.tar.gz
```

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
