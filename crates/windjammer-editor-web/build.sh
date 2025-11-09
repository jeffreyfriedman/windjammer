#!/bin/bash
# Build script for Windjammer Web Editor

set -e

echo "🔨 Building Windjammer Web Editor..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack is not installed"
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

# Build for web
echo "📦 Building WASM package..."
wasm-pack build --target web --release --out-dir pkg

echo "✅ Build complete!"
echo ""
echo "To run the editor:"
echo "  1. Start a local server: python3 -m http.server 8080"
echo "  2. Open http://localhost:8080 in your browser"
echo ""
echo "Or use the serve script: ./serve.sh"

