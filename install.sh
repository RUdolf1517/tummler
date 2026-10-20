#!/bin/sh
# tummler — установка одной командой:
#   curl -fsSL https://raw.githubusercontent.com/RUdolf1517/tummler/main/install.sh | sh
set -eu

REPO="${TUMMLER_REPO:-RUdolf1517/tummler}"
BRANCH="${TUMMLER_BRANCH:-main}"
INSTALL_DIR="${TUMMLER_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
err() { printf 'tummler-install: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1; }

os=$(uname -s)
arch=$(uname -m)
case "$os" in
  Linux)  triple_os=unknown-linux-gnu ;;
  Darwin) triple_os=apple-darwin ;;
  *) err "неподдерживаемая ОС: $os (нужны Linux или macOS)" ;;
esac
case "$arch" in
  x86_64|amd64)  triple_arch=x86_64 ;;
  arm64|aarch64) triple_arch=aarch64 ;;
  *) err "неподдерживаемая архитектура: $arch" ;;
esac

asset="tummler-${triple_arch}-${triple_os}.tar.gz"
mkdir -p "$INSTALL_DIR"

download() {
  if need curl; then
    curl -fsSL "$1" -o "$2"
  elif need wget; then
    wget -qO "$2" "$1"
  else
    return 1
  fi
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 1) Готовый бинарник с GitHub Releases
url="https://github.com/${REPO}/releases/latest/download/${asset}"
if download "$url" "$tmp/$asset" 2>/dev/null; then
  tar -C "$tmp" -xzf "$tmp/$asset"
  bin=$(find "$tmp" -type f -name tummler | head -n 1)
  if [ -n "$bin" ] && [ -f "$bin" ]; then
    chmod +x "$bin"
    mv "$bin" "$INSTALL_DIR/tummler"
    say "установлен $INSTALL_DIR/tummler"
    case ":$PATH:" in
      *":$INSTALL_DIR:"*) ;;
      *) say "добавьте в PATH: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
    esac
    "$INSTALL_DIR/tummler" --version || true
    exit 0
  fi
fi

# 2) Сборка из git через cargo
if need cargo; then
  say "релиз не найден — собираю из исходников (cargo)…"
  cargo install --git "https://github.com/${REPO}" --branch "$BRANCH" --force
  exit 0
fi

err "нужен cargo (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)
или GitHub Release: https://github.com/${REPO}/releases"
