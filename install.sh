#!/bin/sh
# jax-daemon install script
# Usage: curl -fsSL https://raw.githubusercontent.com/jax-protocol/jax-fs/main/install.sh | sh
#        curl -fsSL ... | sh -s -- --version 0.1.9
set -eu

REPO="jax-protocol/jax-fs"
BINARY="jax-daemon"
INSTALL_DIR="${JAX_INSTALL_DIR:-$HOME/.local/bin}"

# Parse arguments
VERSION=""
while [ $# -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --version=*)
      VERSION="${1#--version=}"
      shift
      ;;
    --help|-h)
      echo "Usage: install.sh [--version VERSION]"
      echo ""
      echo "Install or update jax-daemon from GitHub releases."
      echo ""
      echo "Options:"
      echo "  --version VERSION  Install a specific version (default: latest)"
      echo ""
      echo "Environment:"
      echo "  JAX_INSTALL_DIR    Installation directory (default: ~/.local/bin)"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# Detect OS
detect_os() {
  case "$(uname -s)" in
    Darwin) echo "darwin" ;;
    Linux)  echo "linux" ;;
    *)
      echo "Unsupported OS: $(uname -s)" >&2
      exit 1
      ;;
  esac
}

# Detect architecture
detect_arch() {
  case "$(uname -m)" in
    arm64|aarch64) echo "arm64" ;;
    x86_64|amd64)  echo "x64" ;;
    *)
      echo "Unsupported architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

# Validate platform support
if [ "$OS" = "linux" ] && [ "$ARCH" = "arm64" ]; then
  echo "Error: Linux ARM64 binaries are not yet available." >&2
  echo "Install from source: cargo install jax-daemon" >&2
  exit 1
fi

echo "Detected platform: ${OS}/${ARCH}"

# Resolve version
if [ -z "$VERSION" ]; then
  echo "Fetching latest version..."
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" \
    | grep -o '"tag_name": *"jax-daemon-v[^"]*"' \
    | head -1 \
    | sed 's/.*"jax-daemon-v\([^"]*\)".*/\1/')

  if [ -z "$VERSION" ]; then
    echo "Error: Could not determine latest version." >&2
    echo "Check https://github.com/${REPO}/releases for available versions." >&2
    exit 1
  fi
fi

echo "Installing jax-daemon v${VERSION}..."

# Build download URL
ARTIFACT="${BINARY}-${OS}-${ARCH}-${VERSION}"
URL="https://github.com/${REPO}/releases/download/jax-daemon-v${VERSION}/${ARTIFACT}"

# Download binary
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "Downloading ${URL}..."
if ! curl -fSL --progress-bar -o "${TMPDIR}/jax" "$URL"; then
  echo "Error: Download failed." >&2
  echo "Check that version ${VERSION} exists at:" >&2
  echo "  https://github.com/${REPO}/releases/tag/jax-daemon-v${VERSION}" >&2
  exit 1
fi

chmod +x "${TMPDIR}/jax"

# Install
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/jax" "${INSTALL_DIR}/jax"

echo "Installed jax to ${INSTALL_DIR}/jax"

# Check PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    ;;
  *)
    echo ""
    echo "Add ${INSTALL_DIR} to your PATH:"
    echo ""
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
    echo "Add the above line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
    ;;
esac

echo ""
echo "Run 'jax --help' to get started."
