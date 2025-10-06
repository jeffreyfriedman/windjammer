#!/bin/bash
# Windjammer Installation Script
# Builds and installs Windjammer from source

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║   Windjammer Installation Script      ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Rust is not installed${NC}"
    echo "Please install Rust from https://rustup.rs/"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo -e "${GREEN}✓${NC} Rust found: $(rustc --version)"
echo ""

# Build Windjammer
echo -e "${YELLOW}Building Windjammer (this may take a few minutes)...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Build successful"
echo ""

# Determine installation directory
INSTALL_DIR="/usr/local/bin"
STDLIB_DIR="/usr/local/lib/windjammer/std"

# Check if we need sudo
if [ -w "$INSTALL_DIR" ]; then
    SUDO=""
else
    SUDO="sudo"
    echo -e "${YELLOW}Note: Installation requires sudo access${NC}"
fi

# Install binary
echo "Installing windjammer to $INSTALL_DIR..."
$SUDO cp target/release/windjammer "$INSTALL_DIR/windjammer"
$SUDO chmod +x "$INSTALL_DIR/windjammer"

# Install standard library
echo "Installing standard library to $STDLIB_DIR..."
$SUDO mkdir -p "$STDLIB_DIR"
$SUDO cp -r std/* "$STDLIB_DIR/"

# Set environment variable hint
echo ""
echo -e "${GREEN}✓ Installation complete!${NC}"
echo ""
echo "Windjammer has been installed to: $INSTALL_DIR/windjammer"
echo "Standard library installed to: $STDLIB_DIR"
echo ""
echo -e "${YELLOW}Optional:${NC} Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
echo "  export WINDJAMMER_STDLIB=$STDLIB_DIR"
echo ""

# Verify installation
if command -v windjammer &> /dev/null; then
    echo -e "${GREEN}✓${NC} Verification successful"
    windjammer --version
    echo ""
    echo "Try it out:"
    echo "  windjammer --help"
    echo "  windjammer build --path examples/01_basics"
else
    echo -e "${RED}✗ Installation verification failed${NC}"
    echo "Please ensure $INSTALL_DIR is in your PATH"
    exit 1
fi

echo ""
echo -e "${GREEN}Happy coding! 🚀${NC}"
