#!/usr/bin/env bash
set -e

# MCC installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/lyledean1/mcc/main/install.sh | bash

REPO="lyledean1/mcc"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

main() {
    echo "Installing MCC..."
    echo

    # Detect OS and architecture
    OS=$(get_os)
    ARCH=$(get_arch)

    if [ -z "$OS" ] || [ -z "$ARCH" ]; then
        echo "Error: Unsupported platform"
        echo "OS: $(uname -s)"
        echo "Architecture: $(uname -m)"
        exit 1
    fi

    PLATFORM="${OS}-${ARCH}"
    echo "Detected platform: $PLATFORM"

    # Get latest release URL
    DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/mcc-${PLATFORM}.tar.gz"

    echo "Downloading from: $DOWNLOAD_URL"

    # Create temporary directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT

    # Download and extract
    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR"
    else
        echo "Error: Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Check if we need sudo
    if [ -w "$INSTALL_DIR" ]; then
        mv "$TMP_DIR/mcc" "$INSTALL_DIR/mcc"
    else
        echo "Installing to $INSTALL_DIR (requires sudo)..."
        sudo mv "$TMP_DIR/mcc" "$INSTALL_DIR/mcc"
    fi

    # Make executable
    if [ -w "$INSTALL_DIR/mcc" ]; then
        chmod +x "$INSTALL_DIR/mcc"
    else
        sudo chmod +x "$INSTALL_DIR/mcc"
    fi

    echo
    echo "✓ MCC installed successfully to $INSTALL_DIR/mcc"
    echo
    echo "Run 'mcc help' to get started"
}

get_os() {
    case "$(uname -s)" in
        Darwin*)
            echo "macos"
            ;;
        Linux*)
            echo "linux"
            ;;
        *)
            echo ""
            ;;
    esac
}

get_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "amd64"
            ;;
        aarch64|arm64)
            echo "arm64"
            ;;
        *)
            echo ""
            ;;
    esac
}

main "$@"
