#!/usr/bin/env bash
#
# Astra - installer for Arch Linux and Arch-based distributions
# (CachyOS, Manjaro, EndeavourOS, Garuda, ...).
#
# Builds Astra from source and installs it system-wide, or installs a
# pre-built AppImage:
#
#   git clone https://github.com/AmanicX/astra.git
#   cd astra
#   ./scripts/install-arch.sh                  # build from source and install
#   ./scripts/install-arch.sh /path/Astra.AppImage   # install a prebuilt AppImage
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN_NAME="astra"
PREFIX="/opt/astra"
BUNDLE_DIR="$ROOT/src-tauri/target/release/bundle"
DESKTOP_FILE="/usr/share/applications/astra.desktop"

info() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ok\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

[ "$(uname -s)" = "Linux" ] || die "This installer is for Linux only."

DISTRO_ID=""
DISTRO_LIKE=""
if [ -r /etc/os-release ]; then
  . /etc/os-release
  DISTRO_ID="${ID:-}"
  DISTRO_LIKE="${ID_LIKE:-}"
fi
case " $DISTRO_ID $DISTRO_LIKE " in
  *arch*) : ;;
  *) die "This installer targets Arch Linux / Arch-based distributions (detected: ${DISTRO_ID:-unknown})." ;;
esac
info "Detected: ${PRETTY_NAME:-${DISTRO_ID:-unknown}}"

if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
elif have sudo; then
  SUDO="sudo"
else
  die "Root is required; run as root or install sudo."
fi

APPIMAGE_SRC=""
if [ "$#" -ge 1 ]; then
  [ -f "$1" ] || die "No such file: $1"
  APPIMAGE_SRC="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
fi

install_runtime_deps() {
  info "Installing runtime dependencies via pacman..."
  $SUDO pacman -Sy --needed --noconfirm \
    webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg \
    pipewire libdbus libxcb libxrandr mesa fuse2 \
    desktop-file-utils
}

install_build_deps() {
  info "Installing build dependencies via pacman..."
  $SUDO pacman -Sy --needed --noconfirm \
    base-devel clang rust nodejs npm \
    curl wget file pkgconf openssl patchelf
  have cargo || die "cargo was not found after installing rust."
  have node || die "node was not found after installing nodejs."
}

build_app() {
  info "Installing npm dependencies..."
  npm ci
  info "Building Astra (AppImage). This can take several minutes..."
  npm run tauri build -- --bundles appimage
  APPIMAGE_SRC="$(ls "$BUNDLE_DIR"/appimage/*.AppImage 2>/dev/null | head -n1 || true)"
  [ -n "$APPIMAGE_SRC" ] || die "No AppImage was produced under $BUNDLE_DIR/appimage"
}

install_icon() {
  local icon_root="/usr/share/icons/hicolor"
  $SUDO install -d "$icon_root/128x128/apps" "$icon_root/256x256/apps" "$icon_root/512x512/apps"
  if [ -f "$ROOT/src-tauri/icons/128x128.png" ]; then
    $SUDO install -m644 "$ROOT/src-tauri/icons/128x128.png" "$icon_root/128x128/apps/astra.png"
  fi
  if [ -f "$ROOT/src-tauri/icons/128x128@2x.png" ]; then
    $SUDO install -m644 "$ROOT/src-tauri/icons/128x128@2x.png" "$icon_root/256x256/apps/astra.png"
  fi
  if [ -f "$ROOT/src-tauri/icons/icon.png" ]; then
    $SUDO install -m644 "$ROOT/src-tauri/icons/icon.png" "$icon_root/512x512/apps/astra.png"
  fi
  $SUDO gtk-update-icon-cache -q -t -f "$icon_root" 2>/dev/null || true
}

install_app() {
  info "Installing Astra to $PREFIX..."
  $SUDO install -d "$PREFIX"
  $SUDO install -m755 "$APPIMAGE_SRC" "$PREFIX/Astra.AppImage"

  $SUDO install -d /usr/bin
  printf '#!/bin/sh\nexec "%s/Astra.AppImage" "$@"\n' "$PREFIX" | $SUDO tee "/usr/bin/$BIN_NAME" >/dev/null
  $SUDO chmod 755 "/usr/bin/$BIN_NAME"

  $SUDO install -d /usr/share/applications
  $SUDO tee "$DESKTOP_FILE" >/dev/null <<DESKTOP
[Desktop Entry]
Type=Application
Name=Astra
Comment=Local-first, permission-safe desktop AI assistant
Exec=$PREFIX/Astra.AppImage
Icon=astra
Terminal=false
Categories=Utility;Productivity;
StartupWMClass=astra
DESKTOP
  $SUDO chmod 644 "$DESKTOP_FILE"
  $SUDO update-desktop-database /usr/share/applications 2>/dev/null || true

  install_icon
  ok "Installed. Launch Astra from your application menu, or run: $BIN_NAME"
}

info "Astra installer for Arch Linux"
install_runtime_deps
if [ -n "$APPIMAGE_SRC" ]; then
  info "Using prebuilt AppImage: $APPIMAGE_SRC"
else
  install_build_deps
  build_app
fi
install_app
