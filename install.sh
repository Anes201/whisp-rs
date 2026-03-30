#!/usr/bin/env bash
set -euo pipefail

# whisp-rs one-shot installer for Debian/Ubuntu/Pop!_OS
# Usage: curl -sSL https://raw.githubusercontent.com/Anes201/whisp-rs/main/install.sh | bash

REPO="Anes201/whisp-rs"
VERSION="0.1.2"
BINARY_URL="https://github.com/${REPO}/releases/download/v${VERSION}/whisp-rs-x86_64-linux.tar.gz"
INSTALL_DIR="/usr/local/bin"

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { echo -e "${GREEN}✓${NC} $*"; }
warn()  { echo -e "${YELLOW}⚠${NC} $*"; }
error() { echo -e "${RED}✗${NC} $*"; exit 1; }
step()  { echo -e "\n${BOLD}── $* ──${NC}"; }

# ── Check OS ──────────────────────────────────────────────
step "Checking system"
if ! command -v apt &>/dev/null; then
    error "This installer requires apt (Debian/Ubuntu/Pop!_OS)"
fi
info "Debian-based system detected"

# ── Install system dependencies ───────────────────────────
step "Installing system dependencies"

DEPS=()
for pkg in alsa-utils ydotool wl-clipboard; do
    if ! dpkg -s "$pkg" &>/dev/null; then
        DEPS+=("$pkg")
    fi
done

if [ ${#DEPS[@]} -gt 0 ]; then
    info "Installing: ${DEPS[*]}"
    sudo apt update -qq
    sudo apt install -y -qq "${DEPS[@]}"
else
    info "System deps already installed"
fi

# Try to install wtype (best injection method for Wayland)
if ! command -v wtype &>/dev/null; then
    if command -v cargo &>/dev/null; then
        info "Installing wtype from source..."
        sudo apt install -y -qq libxkbcommon-dev libwayland-dev 2>/dev/null || true
        cargo install wtype 2>/dev/null && info "wtype installed" || warn "wtype install failed — will use clipboard fallback"
    else
        warn "wtype not found — will use clipboard fallback (install cargo + wtype for best experience)"
    fi
fi

# ── Download whisp-rs binary ─────────────────────────────
step "Downloading whisp-rs v${VERSION}"

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

info "Downloading from GitHub Releases..."
if command -v curl &>/dev/null; then
    curl -sSL -o "$TMPDIR/whisp-rs.tar.gz" "$BINARY_URL"
elif command -v wget &>/dev/null; then
    wget -q -O "$TMPDIR/whisp-rs.tar.gz" "$BINARY_URL"
else
    error "Neither curl nor wget found. Install one: sudo apt install curl"
fi

tar xzf "$TMPDIR/whisp-rs.tar.gz" -C "$TMPDIR"
chmod +x "$TMPDIR/whisp-rs"

info "Installing to ${INSTALL_DIR}/whisp-rs"
sudo mv "$TMPDIR/whisp-rs" "${INSTALL_DIR}/whisp-rs"
info "whisp-rs ${VERSION} installed"

# ── Input group for hotkey access ────────────────────────
step "Checking input group access"
if groups | grep -qw input; then
    info "User already in 'input' group"
else
    warn "Adding user to 'input' group (needed for hotkey detection)"
    sudo usermod -aG input "$USER"
    warn "Log out and back in for group change to take effect"
fi

# ── Start ydotoold if needed ─────────────────────────────
step "Starting ydotoold daemon"
if command -v ydotool &>/dev/null; then
    if pgrep -x ydotoold &>/dev/null; then
        info "ydotoold already running"
    else
        nohup ydotoold --socket-path=/tmp/.ydotool_socket &>/dev/null &
        sleep 0.3
        if pgrep -x ydotoold &>/dev/null; then
            info "ydotoold started"
        else
            warn "ydotoold failed to start — run manually: ydotoold &"
        fi
    fi
fi

# ── Done ─────────────────────────────────────────────────
echo ""
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo -e "${GREEN}  whisp-rs v${VERSION} installed successfully!${NC}"
echo -e "${BOLD}═══════════════════════════════════════════${NC}"
echo ""
echo "  Run:        whisp-rs"
echo "  Setup:      whisp-rs --setup"
echo "  Set key:    whisp-rs --set-api-key <key>"
echo ""
echo "  Get a free Deepgram API key:"
echo "  https://console.deepgram.com/signup"
echo ""
echo "  Support:    https://ko-fi.com/anes201"
echo ""
