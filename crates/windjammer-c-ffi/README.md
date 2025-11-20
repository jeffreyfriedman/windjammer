# Windjammer C FFI

C Foreign Function Interface (FFI) bindings for the Windjammer Game Framework.

## Overview

This crate provides a C-compatible API layer that enables multi-language support for Windjammer. It's the foundation for all language SDKs (Python, JavaScript, C#, C++, Go, Java, Kotlin, Lua, Swift, Ruby).

## Features

- ✅ **C-Compatible API**: All functions use C ABI
- ✅ **Opaque Pointers**: Type-safe handles for internal objects
- ✅ **Error Handling**: Comprehensive error codes and messages
- ✅ **Memory Safety**: Proper ownership and lifetime management
- ✅ **Panic Safety**: All panics caught at FFI boundary
- ✅ **Auto-Generated Headers**: C/C++ headers via cbindgen

## Architecture

```
┌─────────────────────────────────────────────────┐
│           Language SDKs (12 languages)          │
│  Python │ JS/TS │ C# │ C++ │ Go │ Java │ etc.  │
└────────────────────┬────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────┐
│              C FFI Layer (this crate)           │
│  - Opaque handles                               │
│  - Error handling                               │
│  - Memory management                            │
│  - Type conversions                             │
└────────────────────┬────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────┐
│         Windjammer Game Framework (Rust)        │
│  - ECS, Rendering, Physics, Audio, etc.         │
└─────────────────────────────────────────────────┘
```

## Building

### Rust Library

```bash
cargo build --release
```

This generates:
- `libwindjammer_c_ffi.so` (Linux)
- `libwindjammer_c_ffi.dylib` (macOS)
- `windjammer_c_ffi.dll` (Windows)
- `libwindjammer_c_ffi.a` (static library)

### C Headers

Headers are automatically generated during build:

```bash
cargo build
```

Output: `include/windjammer.h`

## Usage

### From C

```c
#include "windjammer.h"

int main() {
    // Create engine
    WjEngine* engine = wj_engine_new();
    if (!engine) {
        fprintf(stderr, "Failed to create engine: %s\n", wj_get_last_error());
        return 1;
    }
    
    // Create window
    WjWindow* window = wj_window_new("My Game", 800, 600);
    
    // Run game loop
    WjErrorCode result = wj_engine_run(engine);
    
    // Cleanup
    wj_window_free(window);
    wj_engine_free(engine);
    
    return result == WJ_ERROR_CODE_OK ? 0 : 1;
}
```

### From C++

```cpp
#include "windjammer.h"
#include <memory>

// RAII wrapper
struct EngineDeleter {
    void operator()(WjEngine* e) { wj_engine_free(e); }
};

using EnginePtr = std::unique_ptr<WjEngine, EngineDeleter>;

int main() {
    EnginePtr engine(wj_engine_new());
    if (!engine) {
        std::cerr << "Failed to create engine: " 
                  << wj_get_last_error() << std::endl;
        return 1;
    }
    
    // Use engine...
    
    return 0;
}
```

### From Other Languages

Don't use this directly! Use the language-specific SDKs:

- **Python**: `pip install windjammer-sdk`
- **JavaScript**: `npm install @windjammer/sdk`
- **C#**: `dotnet add package Windjammer.SDK`
- **Go**: `go get github.com/windjammer/sdk-go`
- **Java**: Maven/Gradle dependency
- **Kotlin**: Maven/Gradle dependency
- **Lua**: LuaRocks package
- **Swift**: Swift Package Manager
- **Ruby**: `gem install windjammer-sdk`

## API Reference

### Error Handling

```c
// Error codes
typedef enum {
    WJ_ERROR_CODE_OK = 0,
    WJ_ERROR_CODE_NULL_POINTER = 1,
    WJ_ERROR_CODE_INVALID_HANDLE = 2,
    WJ_ERROR_CODE_OUT_OF_MEMORY = 3,
    WJ_ERROR_CODE_INVALID_ARGUMENT = 4,
    WJ_ERROR_CODE_OPERATION_FAILED = 5,
    WJ_ERROR_CODE_PANIC = 6,
} WjErrorCode;

// Get last error message
const char* wj_get_last_error();

// Clear last error
void wj_clear_last_error();
```

### Math Types

```c
// 2D Vector
typedef struct {
    float x, y;
} WjVec2;

WjVec2 wj_vec2_new(float x, float y);

// 3D Vector
typedef struct {
    float x, y, z;
} WjVec3;

WjVec3 wj_vec3_new(float x, float y, float z);

// 4D Vector
typedef struct {
    float x, y, z, w;
} WjVec4;

WjVec4 wj_vec4_new(float x, float y, float z, float w);

// Color (RGBA)
typedef struct {
    float r, g, b, a;
} WjColor;

WjColor wj_color_new(float r, float g, float b, float a);
```

### Engine Management

```c
// Create engine
WjEngine* wj_engine_new();

// Destroy engine
void wj_engine_free(WjEngine* engine);

// Run engine (blocking)
WjErrorCode wj_engine_run(WjEngine* engine);
```

### Window Management

```c
// Create window
WjWindow* wj_window_new(const char* title, uint32_t width, uint32_t height);

// Destroy window
void wj_window_free(WjWindow* window);
```

### Entity Management

```c
// Create entity
WjEntity* wj_entity_new(WjWorld* world);

// Destroy entity
void wj_entity_free(WjEntity* entity);
```

## Memory Management

### Rules

1. **Ownership**: Caller owns returned pointers
2. **Cleanup**: Always call corresponding `_free()` function
3. **Null Checks**: Always check for NULL returns
4. **Error Handling**: Check error codes and messages

### Example

```c
// Create (caller owns)
WjEngine* engine = wj_engine_new();
if (!engine) {
    // Handle error
    return;
}

// Use...

// Destroy (caller must free)
wj_engine_free(engine);
```

## Thread Safety

- ✅ **Thread-Safe**: Error handling (thread-local)
- ⚠️ **Not Thread-Safe**: Most other functions (use from single thread)

For multi-threaded games, create separate engine instances per thread or use external synchronization.

## Performance

- **Zero-Copy**: Math types passed by value (no heap allocation)
- **Minimal Overhead**: Direct function calls, no vtables
- **Panic Safety**: Panics caught, minimal performance impact
- **Optimized**: Release builds use LTO and aggressive optimization

## Testing

```bash
# Run tests
cargo test

# Run with sanitizers
RUSTFLAGS="-Z sanitizer=address" cargo +nightly test
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0

## Status

🚧 **In Development** - API may change

Current version: 0.1.0

