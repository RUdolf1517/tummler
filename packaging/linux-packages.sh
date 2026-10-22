#!/bin/sh
# Build .deb / .rpm from the Linux release tarballs. Used by .github/workflows/release.yml.
# Usage: packaging/linux-packages.sh VERSION DIST_DIR MAN_PAGE
set -eu

VERSION="${1:?version}"
DIST="${2:?dist dir}"
MAN="${3:?man page}"
NFPM_VER="${NFPM_VER:-2.48.0}"

if ! ls "$DIST"/tummler-*-unknown-linux-gnu.tar.gz >/dev/null 2>&1; then
  echo "linux-packages: no Linux tarballs in $DIST, skip"
  exit 0
fi

need() { command -v "$1" >/dev/null 2>&1; }

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

nfpm="$workdir/nfpm"
url="https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VER}/nfpm_${NFPM_VER}_Linux_x86_64.tar.gz"
if need curl; then
  curl -fsSL "$url" | tar -xz -C "$workdir" nfpm
elif need wget; then
  wget -qO- "$url" | tar -xz -C "$workdir" nfpm
else
  echo "linux-packages: need curl or wget to fetch nfpm" >&2
  exit 1
fi
chmod +x "$nfpm"

# Completions are arch-independent; generate from the host-runnable x86_64 binary.
comp="$workdir/completions"
mkdir -p "$comp"
native="$DIST/tummler-x86_64-unknown-linux-gnu.tar.gz"
if [ -f "$native" ]; then
  tar -xzf "$native" -C "$workdir"
  hostbin=$(find "$workdir" -maxdepth 1 -type f -name tummler | head -n 1)
  if [ -n "$hostbin" ]; then
    chmod +x "$hostbin"
    "$hostbin" --completions bash >"$comp/tummler.bash"
    "$hostbin" --completions zsh >"$comp/_tummler"
    "$hostbin" --completions fish >"$comp/tummler.fish"
  fi
  rm -f "$workdir/tummler"
fi

pack_one() {
  triple=$1
  deb_arch=$2
  rpm_name=$3
  tarball="$DIST/tummler-${triple}.tar.gz"
  [ -f "$tarball" ] || return 0

  pkg="$workdir/pkg-$deb_arch"
  rm -rf "$pkg"
  mkdir -p "$pkg"
  tar -xzf "$tarball" -C "$pkg"
  bin=$(find "$pkg" -type f -name tummler | head -n 1)
  [ -n "$bin" ] || return 0
  chmod +x "$bin"

  yaml="$pkg/nfpm.yaml"
  {
    echo "name: tummler"
    echo "arch: $deb_arch"
    echo "platform: linux"
    echo "version: $VERSION"
    echo "release: \"1\""
    echo "section: utils"
    echo "priority: optional"
    echo "maintainer: RUdolf1517 <rudolfzinovev@protonmail.com>"
    echo "description: One CLI that mixes any file into any format"
    echo "homepage: https://github.com/RUdolf1517/tummler"
    echo "license: MIT"
    echo "contents:"
    echo "  - src: $bin"
    echo "    dst: /usr/bin/tummler"
    echo "    file_info:"
    echo "      mode: 0755"
    echo "  - src: $MAN"
    echo "    dst: /usr/share/man/man1/tummler.1"
    if [ -f LICENSE ]; then
      echo "  - src: LICENSE"
      echo "    dst: /usr/share/doc/tummler/copyright"
    fi
    if [ -f "$comp/tummler.bash" ]; then
      echo "  - src: $comp/tummler.bash"
      echo "    dst: /usr/share/bash-completion/completions/tummler"
      echo "  - src: $comp/_tummler"
      echo "    dst: /usr/share/zsh/site-functions/_tummler"
      echo "  - src: $comp/tummler.fish"
      echo "    dst: /usr/share/fish/vendor_completions.d/tummler.fish"
    fi
  } >"$yaml"

  "$nfpm" pkg --packager deb --config "$yaml" --target "$DIST/tummler_${deb_arch}.deb"
  "$nfpm" pkg --packager rpm --config "$yaml" --target "$DIST/tummler.${rpm_name}.rpm"
}

pack_one x86_64-unknown-linux-gnu amd64 x86_64
pack_one aarch64-unknown-linux-gnu arm64 aarch64
