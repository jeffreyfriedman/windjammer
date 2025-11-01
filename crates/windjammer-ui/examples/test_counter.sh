#!/bin/bash
# Test script for the counter WASM app

set -e

echo "Building WASM package..."
cd "$(dirname "$0")/.."
wasm-pack build --target web --out-dir examples/pkg

echo ""
echo "✅ Build successful!"
echo ""
echo "To test the counter app:"
echo "  1. cd examples"
echo "  2. python3 -m http.server 8000"
echo "  3. Open http://localhost:8000/counter_wasm.html in your browser"
echo ""
echo "Or use the Windjammer HTTP server:"
echo "  wj run examples/serve_wasm.wj"


