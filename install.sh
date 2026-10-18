#!/usr/bin/env bash
set -euo pipefail

# sltop instant installer
# Crafted with care by Scoppiato LAB (https://scoppiato.it)

REPO="Scoppiato-LAB/SLTOP"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="sltop"

echo "================================================="
echo "   sltop - Real-time terminal system monitor     "
echo "           Crafted by Scoppiato LAB              "
echo "================================================="

# Detect OS
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
  linux*)
    OS_TYPE="linux"
    ;;
  darwin*)
    OS_TYPE="darwin"
    ;;
  *)
    echo "[-] Unsupported operating system: $OS"
    exit 1
    ;;
esac

# Detect Architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)
    ARCH_TYPE="amd64"
    ;;
  aarch64|arm64)
    ARCH_TYPE="arm64"
    ;;
  *)
    echo "[-] Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

echo "[+] Detected system: ${OS_TYPE}-${ARCH_TYPE}"

# Fetch latest release download URL
echo "[+] Checking latest release from GitHub..."
TAG=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)

if [ -z "$TAG" ]; then
  # Fallback to default version if no release exists yet
  TAG="v0.2.0"
fi

ASSET_NAME="sltop-${OS_TYPE}-${ARCH_TYPE}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET_NAME}"

echo "[+] Downloading ${ASSET_NAME} (${TAG})..."
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if ! curl -f -sSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${ASSET_NAME}"; then
  echo "[-] Download failed. URL was: $DOWNLOAD_URL"
  echo "[-] Please ensure a release with asset '${ASSET_NAME}' exists on https://github.com/${REPO}/releases"
  exit 1
fi

echo "[+] Extracting archive..."
tar -xzf "${TMP_DIR}/${ASSET_NAME}" -C "$TMP_DIR"

echo "[+] Installing ${BINARY_NAME} to ${INSTALL_DIR}..."
if [ -w "$INSTALL_DIR" ]; then
  install -m 755 "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
else
  sudo install -m 755 "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
fi

echo "================================================="
echo " [✓] sltop installed successfully!"
echo " Run 'sltop' to start monitoring."
echo "================================================="
