# SDK FFI Integration Guide 🔌

**How to Connect Language SDKs to the C FFI Layer**

---

## Overview

This guide explains how to integrate each of the 12 language SDKs with the completed C FFI layer. The C FFI provides **145 functions** across **11 modules** that expose the entire Windjammer game framework to other languages.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│              Language SDK (Python)              │
│  - High-level API (Pythonic)                    │
│  - Type hints, docstrings                       │
│  - Error handling (exceptions)                  │
└────────────────────┬────────────────────────────┘
                     │ ctypes/cffi
                     ▼
┌─────────────────────────────────────────────────┐
│              C FFI Layer (145 functions)        │
│  - Opaque handles                               │
│  - Error codes                                  │
│  - C-compatible types                           │
└────────────────────┬────────────────────────────┘
                     │ Direct calls
                     ▼
┌─────────────────────────────────────────────────┐
│         Windjammer Game Framework (Rust)        │
│  - ECS, Rendering, Physics, Audio, etc.         │
└─────────────────────────────────────────────────┘
```

---

## General Integration Steps

### 1. Load the C FFI Library
Each language needs to load `libwindjammer_c_ffi.so` (Linux), `libwindjammer_c_ffi.dylib` (macOS), or `windjammer_c_ffi.dll` (Windows).

### 2. Define C Types
Map C types to native language types:
- `float` → language float
- `int` → language int
- `bool` → language bool
- Opaque pointers → language pointers/handles

### 3. Wrap FFI Functions
Create high-level wrappers that:
- Convert native types to C types
- Call FFI functions
- Check error codes
- Convert C types back to native types
- Throw exceptions on errors

### 4. Implement High-Level API
Create idiomatic classes/functions:
- `Vec2`, `Vec3`, `Color` classes
- `Entity`, `World` classes
- `Camera2D`, `Camera3D` classes
- etc.

---

## Language-Specific Integration

### 1. Python (ctypes/cffi)

#### Loading the Library

```python
from ctypes import *
import os

# Load the library
lib_path = os.path.join(os.path.dirname(__file__), "libwindjammer_c_ffi.so")
lib = CDLL(lib_path)
```

#### Defining C Types

```python
class WjVec2(Structure):
    _fields_ = [
        ("x", c_float),
        ("y", c_float),
    ]

class WjColor(Structure):
    _fields_ = [
        ("r", c_float),
        ("g", c_float),
        ("b", c_float),
        ("a", c_float),
    ]

# Opaque pointer types
WjEntity = c_void_p
WjWorld = c_void_p
```

#### Wrapping Functions

```python
# Define function signatures
lib.wj_vec2_new.argtypes = [c_float, c_float]
lib.wj_vec2_new.restype = WjVec2

lib.wj_entity_spawn.argtypes = [WjWorld]
lib.wj_entity_spawn.restype = WjEntity

# High-level wrapper
class Vec2:
    def __init__(self, x: float, y: float):
        self._handle = lib.wj_vec2_new(c_float(x), c_float(y))
    
    @property
    def x(self) -> float:
        return self._handle.x
    
    @property
    def y(self) -> float:
        return self._handle.y
```

#### Error Handling

```python
class WindjammerError(Exception):
    pass

def check_error():
    error = lib.wj_get_last_error()
    if error:
        msg = string_at(error).decode('utf-8')
        lib.wj_clear_last_error()
        raise WindjammerError(msg)

# Usage
result = lib.wj_some_function(...)
check_error()
```

---

### 2. JavaScript/TypeScript (N-API/ffi-napi)

#### Loading the Library

```javascript
const ffi = require('ffi-napi');
const ref = require('ref-napi');
const Struct = require('ref-struct-napi');

const lib = ffi.Library('libwindjammer_c_ffi', {
  'wj_vec2_new': ['WjVec2', ['float', 'float']],
  'wj_entity_spawn': ['pointer', ['pointer']],
  // ... more functions
});
```

#### Defining C Types

```javascript
const WjVec2 = Struct({
  x: 'float',
  y: 'float',
});

const WjColor = Struct({
  r: 'float',
  g: 'float',
  b: 'float',
  a: 'float',
});

// Opaque pointer types
const WjEntity = ref.refType('void');
const WjWorld = ref.refType('void');
```

#### Wrapping Functions

```typescript
class Vec2 {
  private _handle: any;
  
  constructor(x: number, y: number) {
    this._handle = lib.wj_vec2_new(x, y);
  }
  
  get x(): number {
    return this._handle.x;
  }
  
  get y(): number {
    return this._handle.y;
  }
}
```

---

### 3. C# (P/Invoke)

#### Defining Imports

```csharp
using System;
using System.Runtime.InteropServices;

public static class WindjammerFFI
{
    const string DLL = "windjammer_c_ffi";
    
    [StructLayout(LayoutKind.Sequential)]
    public struct WjVec2
    {
        public float x;
        public float y;
    }
    
    [StructLayout(LayoutKind.Sequential)]
    public struct WjColor
    {
        public float r;
        public float g;
        public float b;
        public float a;
    }
    
    [DllImport(DLL, CallingConvention = CallingConvention.Cdecl)]
    public static extern WjVec2 wj_vec2_new(float x, float y);
    
    [DllImport(DLL, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr wj_entity_spawn(IntPtr world);
}
```

#### Wrapping Functions

```csharp
public class Vec2
{
    private WjVec2 _handle;
    
    public Vec2(float x, float y)
    {
        _handle = WindjammerFFI.wj_vec2_new(x, y);
    }
    
    public float X => _handle.x;
    public float Y => _handle.y;
}
```

---

### 4. C++ (Direct C Linkage)

#### Header Include

```cpp
extern "C" {
    #include "windjammer.h"
}
```

#### Wrapping Functions

```cpp
class Vec2 {
private:
    WjVec2 handle_;
    
public:
    Vec2(float x, float y) : handle_(wj_vec2_new(x, y)) {}
    
    float x() const { return handle_.x; }
    float y() const { return handle_.y; }
};

class Entity {
private:
    WjEntity* handle_;
    
public:
    Entity(World& world) : handle_(wj_entity_spawn(world.handle())) {}
    
    ~Entity() {
        if (handle_) {
            wj_entity_free(handle_);
        }
    }
    
    // Disable copy, enable move
    Entity(const Entity&) = delete;
    Entity& operator=(const Entity&) = delete;
    Entity(Entity&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }
};
```

---

### 5. Go (cgo)

#### Importing

```go
package windjammer

/*
#cgo LDFLAGS: -lwindjammer_c_ffi
#include "windjammer.h"
*/
import "C"
import "unsafe"

type Vec2 struct {
    handle C.WjVec2
}

func NewVec2(x, y float32) Vec2 {
    return Vec2{
        handle: C.wj_vec2_new(C.float(x), C.float(y)),
    }
}

func (v Vec2) X() float32 {
    return float32(v.handle.x)
}

func (v Vec2) Y() float32 {
    return float32(v.handle.y)
}
```

---

### 6. Java (JNI)

#### Native Method Declaration

```java
public class WindjammerFFI {
    static {
        System.loadLibrary("windjammer_c_ffi");
    }
    
    // Native methods
    public static native long wj_vec2_new(float x, float y);
    public static native float wj_vec2_get_x(long handle);
    public static native float wj_vec2_get_y(long handle);
}
```

#### Wrapper Classes

```java
public class Vec2 {
    private long handle;
    
    public Vec2(float x, float y) {
        this.handle = WindjammerFFI.wj_vec2_new(x, y);
    }
    
    public float getX() {
        return WindjammerFFI.wj_vec2_get_x(handle);
    }
    
    public float getY() {
        return WindjammerFFI.wj_vec2_get_y(handle);
    }
}
```

---

### 7. Kotlin (JNI)

Similar to Java, but with Kotlin syntax:

```kotlin
class Vec2(x: Float, y: Float) {
    private val handle: Long = WindjammerFFI.wj_vec2_new(x, y)
    
    val x: Float
        get() = WindjammerFFI.wj_vec2_get_x(handle)
    
    val y: Float
        get() = WindjammerFFI.wj_vec2_get_y(handle)
}
```

---

### 8. Lua (C API)

#### Loading the Library

```lua
local ffi = require("ffi")

ffi.cdef[[
    typedef struct { float x, y; } WjVec2;
    WjVec2 wj_vec2_new(float x, float y);
]]

local lib = ffi.load("windjammer_c_ffi")
```

#### Wrapping Functions

```lua
local Vec2 = {}
Vec2.__index = Vec2

function Vec2:new(x, y)
    local self = setmetatable({}, Vec2)
    self.handle = lib.wj_vec2_new(x, y)
    return self
end

function Vec2:x()
    return self.handle.x
end

function Vec2:y()
    return self.handle.y
end
```

---

### 9. Swift (C Interop)

#### Bridging Header

```swift
import Foundation

// Swift automatically imports C functions
let vec2 = wj_vec2_new(1.0, 2.0)
```

#### Wrapping Functions

```swift
public struct Vec2 {
    private var handle: WjVec2
    
    public init(x: Float, y: Float) {
        self.handle = wj_vec2_new(x, y)
    }
    
    public var x: Float {
        return handle.x
    }
    
    public var y: Float {
        return handle.y
    }
}
```

---

### 10. Ruby (FFI gem)

#### Loading the Library

```ruby
require 'ffi'

module WindjammerFFI
  extend FFI::Library
  ffi_lib 'windjammer_c_ffi'
  
  class WjVec2 < FFI::Struct
    layout :x, :float,
           :y, :float
  end
  
  attach_function :wj_vec2_new, [:float, :float], WjVec2.by_value
end
```

#### Wrapping Functions

```ruby
class Vec2
  def initialize(x, y)
    @handle = WindjammerFFI.wj_vec2_new(x, y)
  end
  
  def x
    @handle[:x]
  end
  
  def y
    @handle[:y]
  end
end
```

---

## Common Patterns

### 1. Resource Management

All opaque handles must be freed:

```python
class Entity:
    def __init__(self, world):
        self._handle = lib.wj_entity_spawn(world._handle)
    
    def __del__(self):
        if self._handle:
            lib.wj_entity_free(self._handle)
```

### 2. Error Checking

Always check for errors after FFI calls:

```python
def call_ffi_function(*args):
    result = lib.some_function(*args)
    error = lib.wj_get_last_error()
    if error:
        msg = string_at(error).decode('utf-8')
        lib.wj_clear_last_error()
        raise WindjammerError(msg)
    return result
```

### 3. Type Conversion

Convert between native and C types:

```python
def to_c_vec2(vec: Vec2) -> WjVec2:
    return WjVec2(c_float(vec.x), c_float(vec.y))

def from_c_vec2(c_vec: WjVec2) -> Vec2:
    return Vec2(c_vec.x, c_vec.y)
```

---

## Testing Strategy

### 1. Unit Tests
Test each wrapper function:

```python
def test_vec2_creation():
    v = Vec2(1.0, 2.0)
    assert v.x == 1.0
    assert v.y == 2.0

def test_entity_spawn():
    world = World()
    entity = world.spawn_entity()
    assert entity is not None
```

### 2. Integration Tests
Test complete workflows:

```python
def test_game_loop():
    app = App()
    world = World()
    entity = world.spawn_entity()
    entity.add_component(Transform2D(Vec2(0, 0)))
    # Run one frame
    app.update(world, 0.016)
```

### 3. Performance Tests
Verify 95%+ native performance:

```python
def test_performance():
    world = World()
    start = time.time()
    for i in range(10000):
        entity = world.spawn_entity()
    elapsed = time.time() - start
    assert elapsed < 0.1  # Should be very fast
```

---

## Build Configuration

### Python
```toml
# pyproject.toml
[build-system]
requires = ["setuptools", "wheel"]

[tool.setuptools.package-data]
windjammer = ["*.so", "*.dylib", "*.dll"]
```

### JavaScript
```json
{
  "dependencies": {
    "ffi-napi": "^4.0.3",
    "ref-napi": "^3.0.3",
    "ref-struct-napi": "^1.1.1"
  }
}
```

### C#
```xml
<ItemGroup>
  <None Include="windjammer_c_ffi.dll">
    <CopyToOutputDirectory>PreserveNewest</CopyToOutputDirectory>
  </None>
</ItemGroup>
```

---

## Next Steps

1. **Implement wrappers** for each language
2. **Test thoroughly** with unit and integration tests
3. **Benchmark performance** to verify 95%+ native
4. **Document API** with language-specific docs
5. **Publish packages** to package managers

---

## Resources

- [C FFI Complete Documentation](FFI_COMPLETE.md)
- [API Reference](API_REFERENCE.md)
- [Quick Start Guide](QUICKSTART.md)

---

*Last Updated: November 20, 2024*

