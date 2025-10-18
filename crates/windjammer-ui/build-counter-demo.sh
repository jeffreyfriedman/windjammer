#!/bin/bash
# Build the interactive counter demo for WASM
set -e

echo "🔨 Building Windjammer UI Counter Demo..."
echo ""

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "❌ wasm-pack not found!"
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

# Build the WASM module
echo "📦 Building WASM module..."
wasm-pack build --target web --dev

echo ""
echo "✅ Build complete!"
echo ""
echo "📂 Output: pkg/windjammer_ui.js"
echo ""
echo "🚀 To run the demo:"
echo "   python3 -m http.server 8000"
echo "   Then open: http://localhost:8000/examples/counter_demo.html"
echo ""
echo "💡 The counter should be fully interactive with:"
echo "   - Increment button (+)"
echo "   - Decrement button (-)"
echo "   - Reset button"
echo "   - Reactive state updates"
echo ""

