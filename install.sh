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

# Use wj self-install for the actual installation
echo -e "${YELLOW}Installing via wj self-install...${NC}"
./target/release/wj self-install

echo ""
echo -e "${GREEN}Happy coding!${NC}"
