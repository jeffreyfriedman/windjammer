# Building the Windjammer C FFI Library

This guide explains how to build the C FFI library for use with multi-language SDKs.

## Prerequisites

- Rust 1.75 or later
- `cbindgen` (for generating C headers)
- Platform-specific C/C++ toolchain

## Build Steps

### 1. Install cbindgen

```bash
cargo install cbindgen
```

### 2. Build the Library

#### Development Build
```bash
cd crates/windjammer-c-ffi
cargo build
```

The library will be in `target/debug/`:
- Linux: `libwindjammer_c_ffi.so`
- macOS: `libwindjammer_c_ffi.dylib`
- Windows: `windjammer_c_ffi.dll`

#### Release Build
```bash
cargo build --release
```

The library will be in `target/release/`.

### 3. Generate C Header

The header is automatically generated during build via `build.rs`:

```bash
# Header will be at:
crates/windjammer-c-ffi/include/windjammer.h
```

Or manually generate:
```bash
cbindgen --config cbindgen.toml --output include/windjammer.h
```

## Using the Library

### Python (ctypes)

```python
from ctypes import CDLL

lib = CDLL("path/to/libwindjammer_c_ffi.so")
```

### JavaScript (ffi-napi)

```javascript
const ffi = require('ffi-napi');
const lib = ffi.Library('libwindjammer_c_ffi', {
  // function declarations
});
```

### C# (P/Invoke)

```csharp
[DllImport("windjammer_c_ffi")]
public static extern int wj_engine_init();
```

### C++

```cpp
extern "C" {
    #include "windjammer.h"
}
```

## Installation

### System-wide (Linux/macOS)

```bash
# Copy library
sudo cp target/release/libwindjammer_c_ffi.* /usr/local/lib/

# Copy header
sudo cp include/windjammer.h /usr/local/include/

# Update library cache (Linux)
sudo ldconfig
```

### SDK Integration

Copy the library to each SDK's directory:

```bash
# Python SDK
cp target/release/libwindjammer_c_ffi.* sdks/python/windjammer_sdk/

# JavaScript SDK
cp target/release/libwindjammer_c_ffi.* sdks/javascript/lib/

# And so on...
```

## Testing

Run the FFI tests:

```bash
cargo test
```

Test with Python SDK:

```bash
cd sdks/python
python examples/hello_world.py
```

## Troubleshooting

### Library Not Found

**Linux/macOS**:
```bash
export LD_LIBRARY_PATH=/path/to/library:$LD_LIBRARY_PATH  # Linux
export DYLD_LIBRARY_PATH=/path/to/library:$DYLD_LIBRARY_PATH  # macOS
```

**Windows**:
Add the library directory to PATH.

### cbindgen Errors

Ensure your Rust code has proper `#[repr(C)]` annotations and `extern "C"` functions.

### Linking Errors

Make sure the Windjammer game framework is built:
```bash
cd crates/windjammer-game-framework
cargo build --release
```

## Development Workflow

1. Make changes to FFI code in `src/`
2. Run tests: `cargo test`
3. Build: `cargo build`
4. Test with SDKs
5. Commit changes

## Cross-Compilation

### For ARM64 (Apple Silicon)

```bash
cargo build --release --target aarch64-apple-darwin
```

### For x86_64

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

## Next Steps

After building the library:
1. Test with Python SDK
2. Test with other language SDKs
3. Run performance benchmarks
4. Create distribution packages

