# Export Target System - Design Document

## Problem

How does the compiler know which backend to use for `@export`?

```windjammer
@export  // WASM? Node.js? Python? C FFI?
fn greet(name: string) -> string { ... }
```

## Design Principles

1. **Default to WASM** - Most common use case for transpiled languages
2. **Auto-detection** - Infer from project structure when possible
3. **Explicit control** - CLI flags for override
4. **Semantic decorators** - `@export` stays implementation-agnostic

## Solution: Multi-layered Target Detection

### Layer 1: CLI Flag (Highest Priority)
```bash
wj build --target wasm main.wj     # Force WASM
wj build --target node main.wj     # Force Node.js FFI
wj build --target python main.wj   # Force Python bindings
wj build --target c main.wj        # Force C FFI
```

### Layer 2: Project Config File
Create `windjammer.toml` in project root:

```toml
[project]
name = "my-app"
target = "wasm"  # or "node", "python", "c"

[build]
optimize = true
```

### Layer 3: Auto-detection from Cargo.toml
```toml
[lib]
crate-type = ["cdylib"]  # → Compiler infers WASM target
# crate-type = ["dylib"]  # → Compiler infers shared library
```

### Layer 4: Default (Lowest Priority)
If nothing else is specified, default to **WASM** (most common use case).

---

## Implementation

### Compiler Target Enum
```rust
pub enum CompilationTarget {
    Wasm,        // Browser/WASM
    Node,        // Node.js native modules
    Python,      // Python FFI via PyO3
    C,           // C FFI
    Native,      // Pure Rust (no FFI)
}
```

### Target Detection Logic
```rust
impl CodeGenerator {
    fn detect_target(&self, cli_args: &CliArgs, project_dir: &Path) -> CompilationTarget {
        // 1. CLI flag (highest priority)
        if let Some(target) = &cli_args.target {
            return target.clone();
        }
        
        // 2. windjammer.toml config
        if let Ok(config) = read_windjammer_config(project_dir) {
            if let Some(target) = config.target {
                return target;
            }
        }
        
        // 3. Auto-detect from Cargo.toml
        if let Ok(cargo_toml) = read_cargo_toml(project_dir) {
            if cargo_toml.has_crate_type("cdylib") {
                return CompilationTarget::Wasm;
            }
        }
        
        // 4. Default to WASM
        CompilationTarget::Wasm
    }
}
```

### Decorator Mapping Per Target
```rust
fn map_decorator(&mut self, name: &str, target: &CompilationTarget) -> String {
    match (name, target) {
        ("export", CompilationTarget::Wasm) => {
            self.needs_wasm_imports = true;
            "wasm_bindgen".to_string()
        }
        ("export", CompilationTarget::Node) => {
            self.needs_neon_imports = true;
            "neon::export".to_string()
        }
        ("export", CompilationTarget::Python) => {
            self.needs_pyo3_imports = true;
            "pyfunction".to_string()
        }
        ("export", CompilationTarget::C) => {
            "no_mangle".to_string()
        }
        ("export", CompilationTarget::Native) => {
            "".to_string()  // No-op for native
        }
        ("test", _) => "test".to_string(),
        (other, _) => other.to_string()
    }
}
```

### Implicit Imports Per Target
```rust
fn generate_implicit_imports(&self, target: &CompilationTarget) -> String {
    match target {
        CompilationTarget::Wasm if self.needs_wasm_imports => {
            "use wasm_bindgen::prelude::*;\n".to_string()
        }
        CompilationTarget::Node if self.needs_neon_imports => {
            "use neon::prelude::*;\n".to_string()
        }
        CompilationTarget::Python if self.needs_pyo3_imports => {
            "use pyo3::prelude::*;\n".to_string()
        }
        _ => String::new()
    }
}
```

---

## Example Usage

### Example 1: WASM (Default)
```windjammer
// main.wj
@export
fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}
```

```bash
wj build main.wj  # Auto-detects WASM from Cargo.toml
```

**Generates**:
```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
```

---

### Example 2: Node.js (Explicit)
```windjammer
// main.wj
@export
fn add(a: int, b: int) -> int {
    a + b
}
```

```bash
wj build --target node main.wj
```

**Generates**:
```rust
use neon::prelude::*;

#[neon::export]
fn add(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let a = cx.argument::<JsNumber>(0)?.value(&mut cx) as i64;
    let b = cx.argument::<JsNumber>(1)?.value(&mut cx) as i64;
    Ok(cx.number(a + b))
}
```

---

### Example 3: Python FFI
```windjammer
// main.wj
@export
fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}
```

```bash
wj build --target python main.wj
```

**Generates**:
```rust
use pyo3::prelude::*;

#[pyfunction]
fn fibonacci(n: i64) -> i64 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

#[pymodule]
fn my_module(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fibonacci, m)?)?;
    Ok(())
}
```

---

### Example 4: C FFI
```windjammer
// main.wj
@export
fn compute(x: f64, y: f64) -> f64 {
    x * x + y * y
}
```

```bash
wj build --target c main.wj
```

**Generates**:
```rust
#[no_mangle]
pub extern "C" fn compute(x: f64, y: f64) -> f64 {
    x * x + y * y
}
```

---

## Target-Specific Features

### WASM
- Auto-inject `wasm_bindgen` imports
- Handle JS types (JsString, Promise, etc.)
- Support `web_sys` and `js_sys`

### Node.js
- Auto-inject `neon` imports
- Handle V8 types (JsNumber, JsString, etc.)
- Support async/await with libuv

### Python
- Auto-inject `pyo3` imports
- Handle Python types (PyList, PyDict, etc.)
- Support `#[pymodule]` generation

### C FFI
- Use `#[no_mangle]` and `extern "C"`
- Ensure C-compatible types
- Generate header files

---

## Optional: Target-Specific Decorators

For advanced users who need fine-grained control:

```windjammer
@export(target: "wasm")
fn wasm_only() { ... }

@export(target: "node")
fn node_only() { ... }

@export(target: ["wasm", "node"])  // Multi-target
fn cross_platform() { ... }
```

But 99% of users should just use `@export` and let the compiler figure it out.

---

## Implementation Phases

### Phase 1 (v0.4.0): WASM Only
- `@export` → `#[wasm_bindgen]`
- Auto-inject WASM imports
- CLI flag `--target wasm` (no-op, but accepted)

### Phase 2 (v0.5.0): Add Node.js
- Implement `--target node`
- Add Neon codegen backend
- Auto-detect from package.json

### Phase 3 (v0.6.0): Add Python & C
- Implement `--target python`
- Implement `--target c`
- Add PyO3 and C FFI backends

### Phase 4 (v1.0.0): Polish
- `windjammer.toml` config file
- Multi-target builds
- Cross-platform stdlib

---

## Configuration File Format

`windjammer.toml`:
```toml
[project]
name = "my-app"
version = "0.1.0"
target = "wasm"  # Default target

[build]
optimize = true
strip_debug = false

[targets.wasm]
features = ["web-sys/Window", "web-sys/Document"]

[targets.node]
module_type = "commonjs"  # or "esm"

[targets.python]
module_name = "my_app"
python_version = "3.9"
```

---

## Migration Path

Users currently using `@wasm_bindgen` can continue to use it (it passes through as-is), but we recommend migrating to `@export` for future compatibility.

**Migration script** (future feature):
```bash
windjammer migrate --from 0.3.0 --to 0.4.0 .
# Automatically replaces @wasm_bindgen with @export
# Removes explicit use statements
```

---

## Benefits

1. **Future-proof**: Can add new targets without breaking changes
2. **Simple defaults**: Most users never need to think about targets
3. **Explicit control**: Power users can override via CLI or config
4. **Semantic code**: `@export` means "make this available externally", regardless of target

---

## Open Questions

1. Should we support multiple targets in one build?
   - Pro: Single codebase, multiple outputs
   - Con: Complex, might confuse users

2. Should we auto-detect target from dependencies?
   - If `Cargo.toml` has `wasm-bindgen`, assume WASM
   - If it has `neon`, assume Node.js

3. Should target be a first-class citizen in the language?
   ```windjammer
   #if target == "wasm"
   @export
   fn browser_only() { ... }
   #endif
   ```

---

**Status**: Design proposal for discussion  
**Target Version**: v0.4.0 (WASM only), v0.5.0+ (other targets)  
**Author**: Windjammer Team  
**Date**: 2025-10-04

