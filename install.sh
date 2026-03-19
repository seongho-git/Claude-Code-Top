#!/usr/bin/env bash
#
# cctop Installation Script
# Developer: Seongho Kim (Yonsei University)
# Email: seongho-kim@yonsei.ac.kr
# GitHub: seongho-git
#

set -e

if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        echo "Error: cargo not found. Please install Rust."
        exit 1
    fi
fi

echo "Building cctop..."
cargo build --release

BIN_PATH="$HOME/.claude-code-top"
EXECUTABLE="target/release/cctop"

echo "Checking $BIN_PATH directory..."
if [ ! -d "$BIN_PATH" ]; then
    echo "Creating $BIN_PATH..."
    mkdir -p "$BIN_PATH"
fi

echo "Installing binary to $BIN_PATH/cctop..."
install -m 755 "$EXECUTABLE" "$BIN_PATH/cctop"

echo ""
echo "========================================"
echo "✅ Installation Complete!"
echo "========================================"
echo ""
echo "To run cctop from anywhere, you need to add the installation directory to your PATH."
echo "Copy and run the following command based on your shell:"
echo ""
echo "[For zsh]"
echo "  echo 'export PATH=\"\$HOME/.claude-code-top:\$PATH\"' >> ~/.zshrc && source ~/.zshrc"
echo ""
echo "[For bash]"
echo "  echo 'export PATH=\"\$HOME/.claude-code-top:\$PATH\"' >> ~/.bashrc && source ~/.bashrc"
echo ""
echo "After adding it to your PATH, you can start the monitor by simply typing 'cctop'."
