#!/bin/bash
# Arca Language Installer
# Usage: curl -sSL https://raw.githubusercontent.com/.../install.sh | bash

set -e

ARCA_VERSION="0.1.0-alpha"
INSTALL_DIR="${HOME}/.arca"
BIN_DIR="${INSTALL_DIR}/bin"
PROFILE_FILE=""

# Detect shell profile
if [ -n "$ZSH_VERSION" ]; then
    PROFILE_FILE="${HOME}/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    PROFILE_FILE="${HOME}/.bashrc"
else
    PROFILE_FILE="${HOME}/.profile"
fi

echo "Installing Arca ${ARCA_VERSION}..."

# Create install directory
mkdir -p "${BIN_DIR}"

# Determine the binary path
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "${SCRIPT_DIR}/target/release/arca" ]; then
    # Running from source
    cp "${SCRIPT_DIR}/target/release/arca-cli" "${BIN_DIR}/arca"
elif [ -f "${SCRIPT_DIR}/target/release/arca-cli" ]; then
    # Binary in same directory
    cp "${SCRIPT_DIR}/target/release/arca-cli" "${BIN_DIR}/arca"
else
    # Download from GitHub releases
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    case "${OS}" in
        Linux*)
            OS="linux"
            ;;
        Darwin*)
            OS="darwin"
            ;;
        *)
            echo "Unsupported OS: ${OS}"
            exit 1
            ;;
    esac
    
    case "${ARCH}" in
        x86_64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            echo "Unsupported architecture: ${ARCH}"
            exit 1
            ;;
    esac
    
    ARCHIVE="arca-${OS}-${ARCH}.tar.gz"
    DOWNLOAD_URL="https://github.com/your-org/arca/releases/latest/download/${ARCHIVE}"
    
    echo "Downloading from ${DOWNLOAD_URL}..."
    curl -L -o "/tmp/${ARCHIVE}" "${DOWNLOAD_URL}"
    tar -xzf "/tmp/${ARCHIVE}" -C "${BIN_DIR}"
    rm -f "/tmp/${ARCHIVE}"
fi

chmod +x "${BIN_DIR}/arca"

# Add to PATH if not already present
PATH_LINE="export PATH=\"${BIN_DIR}:\$PATH\""
if ! grep -q "${BIN_DIR}" "${PROFILE_FILE}" 2>/dev/null; then
    echo "" >> "${PROFILE_FILE}"
    echo "# Arca Language" >> "${PROFILE_FILE}"
    echo "${PATH_LINE}" >> "${PROFILE_FILE}"
    echo "Added to ${PROFILE_FILE}"
else
    echo "PATH already configured in ${PROFILE_FILE}"
fi

# Source the profile
echo ""
echo "Installing shell completions..."
COMPLETION_LINE='eval "$(arca --completion)"'
if ! grep -q 'arca.*completion' "${PROFILE_FILE}" 2>/dev/null; then
    echo "${COMPLETION_LINE}" >> "${PROFILE_FILE}"
fi

echo ""
echo "Installation complete!"
echo ""
echo "Please run the following command to start using Arca:"
echo ""
echo "    source ${PROFILE_FILE}"
echo "    arca --version"
echo ""
echo "Or start a new terminal session."
