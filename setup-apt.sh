#!/usr/bin/env bash
set -euo pipefail

# Setup Scoppiato LAB APT repository for sltop
echo "[+] Adding Scoppiato LAB APT repository..."

REPO_URL="https://scoppiato-lab.github.io/SLTOP"
SOURCES_FILE="/etc/apt/sources.list.d/scoppiato-lab.list"

echo "deb [trusted=yes] ${REPO_URL} stable main" | sudo tee "$SOURCES_FILE" > /dev/null

echo "[+] Updating apt package index..."
sudo apt-get update -o Dir::Etc::sourcelist="sources.list.d/scoppiato-lab.list" -o Dir::Etc::sourceparts="-" -o APT::Get::List-Cleanup="0" || sudo apt update

echo "================================================="
echo " [✓] Scoppiato LAB APT repository configured!"
echo " You can now install and update sltop with:"
echo "     sudo apt install sltop"
echo "================================================="
