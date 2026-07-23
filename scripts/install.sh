#!/bin/bash
# Arca Auto-Update Installer
# Usage: 
#   make install              # Build and install
#   make update              # Update (force reinstall)
#   bash scripts/install.sh --local   # Force local install

set -e

INSTALL_DIR="${HOME}/.arca"
BIN_DIR="${INSTALL_DIR}/bin"
ARCA_VERSION="0.3.2"
FORCE_UPDATE=false
LOCAL_ONLY=false

# Parse flags
for arg in "$@"; do
    case $arg in
        --force|-f) FORCE_UPDATE=true ;;
        --local|-l) LOCAL_ONLY=true ;;
    esac
done

# Detect shell profile
if [ -n "$ZSH_VERSION" ]; then
    PROFILE_FILE="${HOME}/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    PROFILE_FILE="${HOME}/.bashrc"
else
    PROFILE_FILE="${HOME}/.profile"
fi

echo "🔧 Arca Installer"

# Get project root (parent of scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Check existing installation
if [ -f "${BIN_DIR}/arca" ]; then
    CURRENT=$("${BIN_DIR}/arca" --version 2>/dev/null | head -1 | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1 || echo "unknown")
    echo "📦 Current: v${CURRENT}"
    
    if [ "$FORCE_UPDATE" = false ]; then
        echo "✅ Already installed. Use --force to reinstall."
        "${BIN_DIR}/arca" --version
        exit 0
    fi
    echo "⚡ Force reinstall enabled..."
else
    echo "🆕 Fresh installation"
fi

echo "📥 Installing..."

# Install binary
mkdir -p "${BIN_DIR}"

# Priority 1: Local build in target/release/
if [ -f "${PROJECT_ROOT}/target/release/arca-cli" ]; then
    echo "📦 Using local build..."
    cp "${PROJECT_ROOT}/target/release/arca-cli" "${BIN_DIR}/arca"
    chmod +x "${BIN_DIR}/arca"

# Priority 2: Binary in project root
elif [ -f "${PROJECT_ROOT}/arca" ]; then
    echo "📦 Using binary in project root..."
    cp "${PROJECT_ROOT}/arca" "${BIN_DIR}/arca"
    chmod +x "${BIN_DIR}/arca"

# Priority 3: GitHub releases (unless --local)
elif [ "$LOCAL_ONLY" = false ]; then
    OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m)"
    [ "$ARCH" = "arm64" ] && ARCH="aarch64"
    
    URL="https://github.com/your-org/arca/releases/latest/download/arca-${OS}-${ARCH}"
    echo "📥 Downloading from GitHub..."
    
    if curl -L --fail -o "${BIN_DIR}/arca" "${URL}" 2>/dev/null; then
        chmod +x "${BIN_DIR}/arca"
    else
        echo "❌ Download failed."
        echo "   Try: bash scripts/install.sh --local"
        rm -rf "${BIN_DIR}"
        exit 1
    fi
else
    echo "❌ Local build not found."
    echo "   Run: cargo build --release"
    rm -rf "${BIN_DIR}"
    exit 1
fi

# Verify installation
NEW_VERSION=$("${BIN_DIR}/arca" --version 2>/dev/null | head -1 || echo "installed")
echo ""
echo "✅ Installed: ${BIN_DIR}/arca"
echo "   ${NEW_VERSION}"

# Add to PATH
if ! grep -qF "${BIN_DIR}" "${PROFILE_FILE}" 2>/dev/null; then
    echo "" >> "${PROFILE_FILE}"
    echo "# Arca Language" >> "${PROFILE_FILE}"
    echo "export PATH=\"${BIN_DIR}:\$PATH\"" >> "${PROFILE_FILE}"
    echo "📝 Added to PATH in ${PROFILE_FILE}"
fi

echo ""
echo "🚀 Run: source ${PROFILE_FILE} && arca --version"
