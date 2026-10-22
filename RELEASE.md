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

v0.4.0 hashes (fill after the GitHub Release):

```
6e373d95198b8fd601159386c36f454744efc73fe11acb2a1087b32c171771ce  tummler-aarch64-apple-darwin.tar.gz
76ae001e4e600ad52bd811c2f3e4a14997a1559374786935ecd8f04622a60dd4  tummler-aarch64-unknown-linux-gnu.tar.gz
98852b0e076109000cf9c854ef5206bf524c27013a778e84f1fb676c9ddd3ea6  tummler-x86_64-apple-darwin.tar.gz
44e0f2b3326fea74b2c3afc67281917f3ea458559d2e9c4fb92d25aff928b440  tummler-x86_64-unknown-linux-gnu.tar.gz
```

## GitHub About

Description:

```
One CLI that mixes any file into any format. Images, audio, video, documents, archives. Format last: tummler photo.png jpg
```

Topics: `rust` `cli` `converter` `ffmpeg` `image` `transcode` `heic` `webp` `macos` `linux` `pandoc`
