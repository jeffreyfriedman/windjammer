# Windjammer WASM Hello Example ✅

**This example works!** It demonstrates Windjammer code compiling to WebAssembly and running in the browser.

## What It Demonstrates

- ✅ String functions with Unicode support (`greet`)
- ✅ Math operations (`add`)
- ✅ Stateful objects with mutable state (`Counter`)
- ✅ Complete build pipeline: Windjammer → Rust → WASM → Browser

## Quick Start

```bash
cd examples/wasm_hello

# Build the WASM (if not already built)
wasm-pack build --target web

# Serve and open
python3 -m http.server 8000
# or
npx serve .

# Open http://localhost:8000 in your browser
```

## Build From Scratch

If you want to rebuild from Windjammer source:

```bash
# 1. Transpile Windjammer to Rust
cd ../..  # Go to windjammer project root
cargo run -- build --path examples/wasm_hello/main.wj --output examples/wasm_hello/build

# 2. Copy to src/lib.rs and add WASM decorators
cd examples/wasm_hello
mkdir -p src
cp build/main.rs src/lib.rs

# 3. Manually add #[wasm_bindgen] and pub keywords
# Edit src/lib.rs:
#   - Change first line to: use wasm_bindgen::prelude::*;
#   - Add #[wasm_bindgen] before impl Counter
#   - Add #[wasm_bindgen] before standalone functions
#   - Add pub to all public items

# 4. Build WASM
wasm-pack build --target web

# 5. Serve
python3 -m http.server 8000
```

## How It Works

1. **Windjammer Code** (`main.wj`) defines functions and structs
2. **Transpiler** converts to Rust (`src/lib.rs`)
3. **wasm-pack** compiles Rust to WASM
4. **Browser** loads WASM via JavaScript glue code

## File Sizes

- WASM binary: ~18KB  
- JS glue code: ~8KB
- Total download: ~26KB (gzipped: <10KB)

## What This Proves

✅ **Windjammer → Rust transpilation works**
✅ **Generated Rust compiles to WASM**
✅ **WASM runs in browsers with full functionality**
✅ **All codegen fixes are working:**
  - Copy types passed by value
  - Proper slice types
  - Correct operator precedence
  - Float literals formatted correctly

## Troubleshooting

**CORS errors?**
- Make sure you're serving via HTTP (not file://)
- Use `python3 -m http.server` or similar

**Module not found?**
- Check that `pkg/` directory exists
- Run `wasm-pack build --target web` if missing

**Functions not defined?**
- Make sure all functions in `src/lib.rs` have:
  - `#[wasm_bindgen]` attribute
  - `pub` visibility

## Next Steps

Once you see this working, try the more complex `wasm_game` example (Conway's Game of Life)!

