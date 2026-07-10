#!/bin/bash
# CLUAIZ Core Infrastructure Installer - VERSION 0.1.0
# Industrial Standard Deployment Script

set -euo pipefail

HUB_PATH="${HOME}/.cluaiz"
REPO="cluaiz/cluaiz"

# --- UI Matrix (Industrial) ---
BOLD='\033[1m'; CYAN='\033[0;36m'; GRAY='\033[0;90m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'; RED='\033[0;31m'; NC='\033[0m'

write_step() { echo -ne "  ${GRAY}[ ] $1...${NC}"; }
complete_step() { echo -e "\r  ${GREEN}[✓]${NC} $1   "; }
write_success() { echo -e "\n  ${GREEN}[OK] $1${NC}"; }
write_error() { echo -e "\n  ${RED}[ERR] $1${NC}"; }

# --- Header ---
clear
echo -e "\n  ${BOLD}CLUAIZ CORE INFRASTRUCTURE (V0.1.0)${NC}"
echo -e "  ${GRAY}Industrial Deployment Sequence${NC}\n"

# 1. Environment Provisioning
write_step "Provisioning environment"
mkdir -p "$HUB_PATH/bin" "$HUB_PATH/apps/cli" "$HUB_PATH/engine" "$HUB_PATH/interface-engines/kernels" "$HUB_PATH/interface-engines/drivers"
complete_step "Provisioning environment"

# 2. System Integration
if [[ ":$PATH:" != *":$HUB_PATH/bin:"* ]]; then
    SHELL_RC="$HOME/.bashrc"
    [[ "$SHELL" == *"zsh"* ]] && SHELL_RC="$HOME/.zshrc"
    if ! grep -q "CLUAIZ_ROOT" "$SHELL_RC" 2>/dev/null; then
        echo -e "\n# CLUAIZ Environment\nexport CLUAIZ_ROOT=\"$HUB_PATH\"\nexport PATH=\"\$PATH:$HUB_PATH/bin\"" >> "$SHELL_RC"
    fi
    export CLUAIZ_ROOT="$HUB_PATH"
    export PATH="$PATH:$HUB_PATH/bin"
fi

# 3. Artifact Retrieval
write_step "Resolving artifacts"
ALL_RELEASES=$(curl -s "https://api.github.com/repos/$REPO/releases")
complete_step "Resolving artifacts"

OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH_TYPE=$(uname -m)
case "$OS_TYPE" in
    linux) OS="linux"; EXT="so" ;;
    darwin) OS="mac"; EXT="dylib" ;;
    *) write_error "Unsupported OS"; exit 1 ;;
esac
case "$ARCH_TYPE" in
    x86_64) ARCH="x64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) write_error "Unsupported Arch"; exit 1 ;;
esac
PLATFORM="$OS-$ARCH"

# --- CLI ---
CLI_URL=$(echo "$ALL_RELEASES" | grep -oE '"browser_download_url": "[^"]+cluaiz-dev-release-'"$PLATFORM"'"' | head -1 | cut -d'"' -f4 || echo "")
if [ -z "$CLI_URL" ]; then
    CLI_URL=$(echo "$ALL_RELEASES" | grep -oE '"browser_download_url": "[^"]+cluaiz-[^"]+-'"$PLATFORM"'"' | head -1 | cut -d'"' -f4 || echo "")
fi

write_step "Retrieving CLI ($PLATFORM)"
curl -sL "$CLI_URL" -o "$HUB_PATH/apps/cli/cluaiz"
chmod +x "$HUB_PATH/apps/cli/cluaiz"
ln -sf "$HUB_PATH/apps/cli/cluaiz" "$HUB_PATH/bin/cluaiz"
complete_step "Retrieving CLI ($PLATFORM)"

# --- Engine ---
ENGINE_URL=$(echo "$ALL_RELEASES" | grep -oE '"browser_download_url": "[^"]+cluaiz-engine-dev-release-'"$PLATFORM"'\.'"$EXT"'"' | head -1 | cut -d'"' -f4 || echo "")
if [ -z "$ENGINE_URL" ]; then
    ENGINE_URL=$(echo "$ALL_RELEASES" | grep -oE '"browser_download_url": "[^"]+cluaiz-engine-[^"]+-'"$PLATFORM"'\.'"$EXT"'"' | head -1 | cut -d'"' -f4 || echo "")
fi

write_step "Retrieving Core Engine"
curl -sL "$ENGINE_URL" -o "$HUB_PATH/engine/cluaiz-engine.$EXT"
complete_step "Retrieving Core Engine"

# --- Default Kernel ---
if [[ "$OS" == "mac" ]]; then
    KERNEL_URL=$(echo "$ALL_RELEASES" | grep -oE '"browser_download_url": "[^"]+cluaiz-kernel-dev-release-'"$PLATFORM"'\.dylib"' | head -1 | cut -d'"' -f4 || echo "")
else
    # Linux
    if [[ "$ARCH" == "arm64" ]]; then
        TARGET_PLATFORM="linux-arm64"
    else
        HAS_AVX512=$(grep -o "avx512f" /proc/cpuinfo | head -1 || echo "")
        if [ -n "$HAS_AVX512" ]; then
            TARGET_PLATFORM="linux-x64-avx512"
        else
            TARGET_PLATFORM="linux-x64-avx2"
        fi
    fi
    KERNEL_URL=$(echo "$ALL_RELEASES" | grep -oE '"browser_download_url": "[^"]+cluaiz-kernel-dev-release-'"$TARGET_PLATFORM"'\.so"' | head -1 | cut -d'"' -f4 || echo "")
fi

if [ -n "$KERNEL_URL" ]; then
    write_step "Retrieving Core Kernel"
    curl -sL "$KERNEL_URL" -o "$HUB_PATH/interface-engines/kernels/libcluaiz_llama.$EXT"
    complete_step "Retrieving Core Kernel"
fi

write_success "Deployment successful."
echo -e "  Launching CLI...\n"

# Launch CLI
"$HUB_PATH/bin/cluaiz"
