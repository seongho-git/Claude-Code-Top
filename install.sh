#!/usr/bin/env bash
#
# cctop Installation Script
# Developer: Seongho Kim (Yonsei University)
# Email: seongho-kim@yonsei.ac.kr
# GitHub: seongho-git
#

set -e

echo "Checking dependencies..."

if ! command -v claude &> /dev/null; then
    echo "Error: Claude Code not found."
    echo ""
    echo "Install Claude Code with:"
    echo "  curl -fsSL https://claude.ai/install.sh | bash"
    echo ""
    echo "Then re-run this script."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        echo "Error: Rust toolchain not found."
        echo ""
        echo "Install Rust with:"
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        echo "Then reload your shell and re-run this script:"
        echo "  source \"\$HOME/.cargo/env\""
        exit 1
    fi
fi

if ! command -v tmux &> /dev/null; then
    echo "Error: tmux not found (required for background usage tracking)."
    echo ""
    echo "Install tmux with:"
    echo "  macOS:  brew install tmux"
    echo "  Ubuntu: sudo apt install tmux"
    echo ""
    echo "Then re-run this script."
    exit 1
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

# Plan selection
echo ""
echo "========================================"
echo "  Select your Claude plan"
echo "========================================"
echo ""
echo "  1) Pro   — \$18/week,  19k output tokens"
echo "  2) Max5  — \$35/week,  88k output tokens"
echo "  3) Max20 — \$140/week, 220k output tokens"
echo ""
read -p "  Enter choice [1/2/3] (default: 2): " plan_choice

case "$plan_choice" in
    1) plan="pro" ;;
    3) plan="max20" ;;
    *) plan="max5" ;;
esac

echo "{\"plan\": \"$plan\"}" > "$HOME/.cctop.json"
echo "  → Plan '$plan' saved to ~/.cctop.json"

echo ""
echo "========================================"
echo "  Installation Complete!"
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