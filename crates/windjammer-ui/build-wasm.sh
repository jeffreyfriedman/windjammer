#!/bin/bash
# Build script for Windjammer UI WASM packages

set -e

echo "🔨 Building Windjammer UI for WebAssembly..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack is not installed"
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

# Build for web target
echo "📦 Building for web..."
wasm-pack build --target web --out-dir pkg/web

# Build for bundler target (webpack, etc.)
echo "📦 Building for bundlers..."
wasm-pack build --target bundler --out-dir pkg/bundler

# Build for Node.js
echo "📦 Building for Node.js..."
wasm-pack build --target nodejs --out-dir pkg/nodejs

echo "✅ WASM build complete!"
echo ""
echo "Output directories:"
echo "  - pkg/web/      (for vanilla HTML/JS)"
echo "  - pkg/bundler/  (for webpack/rollup/vite)"
echo "  - pkg/nodejs/   (for Node.js)"

