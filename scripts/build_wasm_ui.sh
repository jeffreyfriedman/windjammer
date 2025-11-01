#!/bin/bash
# Build a Windjammer UI app for WASM
# Usage: ./scripts/build_wasm_ui.sh <input.wj> [output_dir]

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <input.wj> [output_dir]"
    echo "Example: $0 examples/ui_counter_simple.wj examples/ui-counter"
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_DIR="${2:-./wasm-output}"
BASENAME=$(basename "$INPUT_FILE" .wj)

echo "🔨 Building Windjammer UI app: $INPUT_FILE"
echo "📦 Output directory: $OUTPUT_DIR"
echo ""

# Step 1: Transpile to Rust
echo "1️⃣  Transpiling Windjammer to Rust..."
./target/release/wj build "$INPUT_FILE" --target rust

# Step 2: Create a WASM-enabled Cargo project
echo "2️⃣  Creating WASM project..."
WASM_PROJECT="$OUTPUT_DIR/wasm-project"
mkdir -p "$WASM_PROJECT/src"

# Create Cargo.toml
cat > "$WASM_PROJECT/Cargo.toml" << 'EOF'
[package]
name = "windjammer-wasm-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
console_error_panic_hook = "0.1"
windjammer-ui = { path = "../../crates/windjammer-ui" }

[profile.release]
opt-level = "z"
lto = true
EOF

# Step 3: Wrap the generated Rust code with WASM bindings
echo "3️⃣  Adding WASM bindings..."
cat > "$WASM_PROJECT/src/lib.rs" << 'EOF'
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Windjammer app starting...".into());
    
    // Call the main function from the generated code
    main();
    
    Ok(())
}

// Include the generated Windjammer code
EOF

# Append the generated code (skip the main function, we'll call it from start)
cat "build/${BASENAME}.rs" | sed 's/^fn main()/fn main_impl()/' | sed 's/println!/web_sys::console::log_1(\&format!/' >> "$WASM_PROJECT/src/lib.rs"

# Fix the main function call
echo "" >> "$WASM_PROJECT/src/lib.rs"
echo "fn main() { main_impl(); }" >> "$WASM_PROJECT/src/lib.rs"

# Step 4: Build with wasm-pack
echo "4️⃣  Building WASM with wasm-pack..."
cd "$WASM_PROJECT"
wasm-pack build --target web --out-dir "../pkg"
cd - > /dev/null

# Step 5: Create HTML file
echo "5️⃣  Creating HTML file..."
cat > "$OUTPUT_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Windjammer App</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        }
        
        #app {
            background: white;
            padding: 3rem;
            border-radius: 16px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            min-width: 300px;
        }
        
        .counter-app {
            text-align: center;
        }
        
        h1 {
            color: #333;
            font-size: 2.5rem;
            margin-bottom: 2rem;
            font-weight: 700;
        }
        
        .buttons {
            display: flex;
            gap: 1rem;
            justify-content: center;
        }
        
        .btn, button {
            background: #667eea;
            color: white;
            border: none;
            padding: 1rem 2rem;
            border-radius: 8px;
            cursor: pointer;
            font-size: 1.5rem;
            font-weight: 600;
            transition: all 0.2s ease;
            min-width: 60px;
        }
        
        .btn:hover, button:hover {
            background: #5568d3;
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
        }
        
        .btn:active, button:active {
            transform: translateY(0);
        }
    </style>
</head>
<body>
    <div id="app">Loading Windjammer app...</div>
    
    <script type="module">
        import init from './pkg/windjammer_wasm_app.js';
        
        async function run() {
            try {
                await init();
                console.log('Windjammer app initialized!');
            } catch (err) {
                console.error('Failed to initialize:', err);
                document.getElementById('app').innerHTML = 
                    '<div style="color: #e53e3e; text-align: center;">' +
                    '<h1>Error Loading App</h1>' +
                    '<p>' + err.message + '</p>' +
                    '</div>';
            }
        }
        
        run();
    </script>
</body>
</html>
EOF

echo ""
echo "✅ Build complete!"
echo ""
echo "📁 Output files:"
echo "   - $OUTPUT_DIR/pkg/         (WASM package)"
echo "   - $OUTPUT_DIR/index.html   (HTML entry point)"
echo ""
echo "🚀 To run:"
echo "   cd $OUTPUT_DIR && python3 -m http.server 8000"
echo "   Then open: http://localhost:8000"
echo ""


