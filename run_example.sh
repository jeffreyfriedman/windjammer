#!/bin/bash
# Run example and copy output files back
EXAMPLE=$1
WORKSPACE=$(pwd)
./target/release/wj build examples/$EXAMPLE/main.wj
cd build
cargo run --release --bin main
# Copy any HTML files back to example directory
find . -name "*.html" -exec cp {} "$WORKSPACE/examples/$EXAMPLE/" \;
