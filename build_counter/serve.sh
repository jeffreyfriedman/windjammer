#!/bin/bash
# Simple HTTP server for testing WASM
cd "$(dirname "$0")"
echo "🌐 Serving Interactive Counter at http://localhost:8000"
echo "📂 Directory: $(pwd)"
echo ""
python3 -m http.server 8000


