# Cut a release

Do this **after** merging to `main`. Binaries first, then Homebrew hashes.

## 1. GitHub Release (no Rust for users)

```bash
git checkout main
git pull
git tag v0.4.0
git push origin v0.4.0
```

Actions builds Apple Silicon, Intel Mac (`x86_64-apple-darwin`), Linux x86_64, Linux ARM tarballs, Windows x64 zip (`tummler-x86_64-pc-windows-msvc.zip`, no ARM), plus `.deb` / `.rpm`, and attaches them to the release. After the tag, copy the Intel Mac SHA-256 into `Formula/tummler.rb` (`on_macos` / `on_intel`). Winget id: `RUdolf1517.tummler` (submit the zip to microsoft/winget-pkgs after the release exists).

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

v0.4.0 hashes:

```
db6de23b52b7fb78825d23415bf8b4311281408452fc9518f98237e55fbcf182  tummler-aarch64-apple-darwin.tar.gz
2a0dfb85e3c57c47902387b68e346338be9272bea36bbf27c493d97e16667243  tummler-aarch64-unknown-linux-gnu.tar.gz
ffef520d550b31edb3f799f5e1378859b72b8ada3c322edbd111e5a7f404d53b  tummler-x86_64-apple-darwin.tar.gz
19de8e44449b75e3485888d950424b5e5ea4526eb8a7f46822c251a6db64dcb8  tummler-x86_64-unknown-linux-gnu.tar.gz
f0491498f8ed64e5f7b897cda5cd0399c8021f5a2a9cbc94fc46172c9617f797  tummler-x86_64-pc-windows-msvc.zip
```

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
