#!/usr/bin/env bash
#
# Astra - one-shot Linux installer.
# Builds Astra from source and installs it on the current machine.
#
#   git clone https://github.com/AmanicX/astra.git
#   cd astra
#   ./scripts/install-linux.sh
#
# Supports Debian/Ubuntu (.deb), Fedora/RHEL (.rpm), and Arch (AppImage),
# plus a generic AppImage fallback for any other distribution.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BIN_NAME="astra"
BUNDLE_DIR="$ROOT/src-tauri/target/release/bundle"

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
  *debian*|*ubuntu*|*mint*|*pop*) FAMILY=debian ;;
  *fedora*|*rhel*|*centos*|*rocky*|*almalinux*) FAMILY=rpm ;;
  *arch*|*manjaro*|*endeavouros*) FAMILY=arch ;;
  *) FAMILY=unknown ;;
esac
info "Detected distro: ${DISTRO_ID:-unknown} (family: $FAMILY)"

if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
elif have sudo; then
  SUDO="sudo"
else
  SUDO=""
  warn "No sudo found; installing system packages may fail."
fi

install_system_deps() {
  case "$FAMILY" in
    debian)
      info "Installing build dependencies via apt (needs sudo)..."
      $SUDO apt-get update
      $SUDO apt-get install -y --no-install-recommends \
        build-essential curl wget file pkg-config libssl-dev \
        libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
        librsvg2-dev patchelf \
        libpipewire-0.3-dev libdbus-1-dev libxcb1-dev libxrandr-dev libgbm-dev
      $SUDO apt-get install -y --no-install-recommends libfuse2 2>/dev/null \
        || $SUDO apt-get install -y --no-install-recommends libfuse2t64 2>/dev/null \
        || warn "libfuse2 not found; the AppImage may need --appimage-extract-and-run."
      ;;
    rpm)
      info "Installing build dependencies via dnf (needs sudo)..."
      $SUDO dnf install -y \
        gcc gcc-c++ make curl wget file pkgconf-pkg-config openssl-devel \
        webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
        librsvg2-devel patchelf fuse \
        pipewire-devel dbus-devel libxcb-devel libXrandr-devel mesa-libgbm-devel
      ;;
    arch)
      info "Installing build dependencies via pacman (needs sudo)..."
      $SUDO pacman -Sy --needed --noconfirm \
        base-devel curl wget file pkgconf openssl \
        webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg patchelf fuse2 \
        pipewire libdbus libxcb libxrandr mesa
      ;;
    *)
      warn "Unknown distro family. Install these, then re-run this script:"
      warn "  webkit2gtk-4.1, gtk3, libappindicator3, librsvg2, patchelf, openssl, a C toolchain"
      ;;
  esac
}

ensure_rust() {
  if have cargo; then
    ok "cargo $(cargo --version | awk '{print $2}')"
  else
    info "Installing the Rust toolchain via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  fi
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  have cargo || die "cargo is still not on PATH after the rustup install."
}

ensure_node() {
  local need=0
  if have node; then
    local major
    major="$(node -p 'process.versions.node.split(".")[0]')"
    if [ "$major" -ge 20 ]; then
      ok "node $(node -v)"
    else
      warn "node $(node -v) is too old (need >= 20)."
      need=1
    fi
  else
    need=1
  fi
  if [ "$need" -eq 1 ]; then
    info "Installing Node.js 20 via nvm..."
    export NVM_DIR="$HOME/.nvm"
    if [ ! -s "$NVM_DIR/nvm.sh" ]; then
      curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.2/install.sh | bash
    fi
    . "$NVM_DIR/nvm.sh"
    nvm install 20
    nvm use 20
  fi
  have node || die "node is not available."
}

build_app() {
  info "Installing npm dependencies..."
  npm ci
  info "Building Astra (release). The first build takes several minutes..."
  npm run tauri build
}

install_appimage() {
  local appimage
  appimage="$(ls "$BUNDLE_DIR"/appimage/*.AppImage 2>/dev/null | head -n1 || true)"
  [ -n "$appimage" ] || die "No AppImage was produced under $BUNDLE_DIR/appimage"

  local opt_dir="$HOME/.local/opt/astra"
  local bin_dir="$HOME/.local/bin"
  local apps_dir="$HOME/.local/share/applications"
  mkdir -p "$opt_dir" "$bin_dir" "$apps_dir"
  cp "$appimage" "$opt_dir/Astra.AppImage"
  chmod +x "$opt_dir/Astra.AppImage"
  ln -sf "$opt_dir/Astra.AppImage" "$bin_dir/$BIN_NAME"

  cat > "$apps_dir/astra.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Astra
Comment=Local-first, permission-safe desktop AI assistant
Exec=$opt_dir/Astra.AppImage
Terminal=false
Categories=Utility;Productivity;
DESKTOP
  chmod +x "$apps_dir/astra.desktop"
  update-desktop-database "$apps_dir" 2>/dev/null || true

  ok "Installed AppImage to $opt_dir/Astra.AppImage"
  case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) warn "Add $bin_dir to your PATH to run '$BIN_NAME' from a terminal." ;;
  esac
}

install_app() {
  case "$FAMILY" in
    debian)
      local deb
      deb="$(ls "$BUNDLE_DIR"/deb/*.deb 2>/dev/null | head -n1 || true)"
      [ -n "$deb" ] || die "No .deb was produced under $BUNDLE_DIR/deb"
      info "Installing $(basename "$deb")..."
      $SUDO apt-get install -y "$deb"
      ;;
    rpm)
      local rpm
      rpm="$(ls "$BUNDLE_DIR"/rpm/*.rpm 2>/dev/null | head -n1 || true)"
      [ -n "$rpm" ] || die "No .rpm was produced under $BUNDLE_DIR/rpm"
      info "Installing $(basename "$rpm")..."
      $SUDO dnf install -y "$rpm"
      ;;
    *)
      install_appimage
      ;;
  esac
}

info "Astra Linux installer"
install_system_deps
ensure_rust
ensure_node
build_app
install_app

echo
ok "Astra is installed. Launch it from your applications menu, or run: $BIN_NAME"
