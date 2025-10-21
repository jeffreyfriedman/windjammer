#!/bin/bash
# Build a Windjammer UI example for WASM
# Usage: ./build-wasm-example.sh <example_name>

set -e

if [ -z "$1" ]; then
    echo "Usage: ./build-wasm-example.sh <example_name>"
    echo "Example: ./build-wasm-example.sh counter_web"
    exit 1
fi

EXAMPLE_NAME=$1
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/../.."

echo "🔨 Building Windjammer UI example: $EXAMPLE_NAME"

# Step 1: Transpile .wj to Rust
echo "📝 Transpiling $EXAMPLE_NAME.wj to Rust..."
cd "$PROJECT_ROOT"
cargo run -- build "crates/windjammer-ui/examples/${EXAMPLE_NAME}.wj" --target wasm

# Step 2: Build with wasm-pack
echo "📦 Building WASM with wasm-pack..."
cd "$SCRIPT_DIR"

# Create a temporary Cargo.toml for the example if needed
# (In production, examples would have their own Cargo.toml)

wasm-pack build --target web --out-dir "pkg/${EXAMPLE_NAME}" --dev

echo "✅ Build complete!"
echo "📂 Output: crates/windjammer-ui/pkg/${EXAMPLE_NAME}/"
echo ""
echo "To run locally:"
echo "  cd crates/windjammer-ui/examples"
echo "  python3 -m http.server 8000"
echo "  Open http://localhost:8000/${EXAMPLE_NAME}.html"

