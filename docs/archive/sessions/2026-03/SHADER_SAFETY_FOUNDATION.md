# Shader Safety Foundation - .wjsl Windjammer Shader Language

**Status:** Implemented (P1)  
**Date:** 2026-03-14  
**Part of:** GAME_ENGINE_IMPROVEMENTS_DESIGN.md Part 1

---

## Overview

The Windjammer Shader Language (`.wjsl`) provides **compile-time type checking** for GPU shaders. It catches host/shader type mismatches before runtime—preventing the black screen bugs documented in `SCREEN_SIZE_TYPE_MISMATCH_BUG.md` and `BLACK_SCREEN_POSTMORTEM.md`.

## Problem Solved

**Before:** WGSL is validated only at GPU driver load time. Host code and shader interface are not type-checked together. A `Vec2<f32>` vs `Vec2<u32>` mismatch causes garbage values, black screens, and hours of debugging.

**After:** `.wjsl` shaders declare their interface explicitly. The compiler validates types at compile time and generates safe WGSL with proper bindings.

---

## Architecture

```
.wjsl source file
    ↓ parse_shader()
ShaderModule AST
    ↓ TypeChecker::check()
Type-checked module
    ↓ generate_wgsl()
WGSL output
```

## File Format (.wjsl)

```wjsl
shader MyComputeShader {
    uniform screen_size: Vec2<f32>
    storage output: array<Vec4<f32>>
}
```

### Supported Types

| .wjsl Type | WGSL Output | Notes |
|------------|-------------|-------|
| `Vec2<f32>` | `vec2<f32>` | Preferred for screen dimensions |
| `Vec2<u32>` | `vec2<u32>` | |
| `Vec3<f32>` | `vec3<f32>` | |
| `Vec4<f32>` | `vec4<f32>` | |
| `Mat4` | `mat4x4<f32>` | |
| `array<T>` | `array<T>` | Storage buffers |
| `StructName` | `StructName` | Custom structs |

### Scalar Types

- `f32` - 32-bit float (WGSL native)
- `f64` - **Rejected** for uniforms (WGSL doesn't support f64 in uniforms)
- `u32` - Unsigned 32-bit
- `i32` - Signed 32-bit
- `bool` - Boolean

---

## CLI Usage

```bash
# Compile .wjsl to WGSL (stdout)
wj shader-compile my_shader.wjsl

# Compile to file
wj shader-compile my_shader.wjsl -o my_shader.wgsl
```

---

## API

### Parse

```rust
use windjammer::shader::parse_shader;

let source = r#"
    shader S {
        uniform screen_size: Vec2<f32>
        storage output: array<Vec4<f32>>
    }
"#;
let module = parse_shader(source)?;
```

### Type Check

```rust
use windjammer::shader::{TypeChecker, parse_shader};

let module = parse_shader(source)?;
let checker = TypeChecker::new();

// Validate host variable matches shader uniform
checker.check_uniform_match(&module, "screen_size", &Type::Vec2(ScalarType::F32))?;

// Validate storage buffer
checker.check_storage_match(&module, "output", &Type::Array(Box::new(Type::Vec4(ScalarType::F32))))?;
```

### Compile (Parse + Type Check + Codegen)

```rust
use windjammer::shader::compile_shader;

let wgsl = compile_shader(source)?;
```

---

## Compile-Time Checks

1. **Host type matches shader type** - `check_uniform_match()` / `check_storage_match()`
2. **No f64 in uniforms** - WGSL doesn't support f64; rejected at compile time
3. **Uniform/storage not found** - Clear error if host references non-existent binding

---

## Error Messages

```
error: Type mismatch at uniform screen_size: expected Vec2<f32>, found Vec2<f64>
```

```
error: Uniform 'screen_size' not found in shader
```

```
error: WGSL does not support f64 - use f32 for uniforms
```

---

## Test Coverage

- **AST:** 3 tests (display, equality)
- **Parser:** 12 tests (uniforms, storage, types, comments, empty, structs)
- **Type Checker:** 16 tests (match, mismatch, not found, f64 rejection, storage)
- **WGSL Codegen:** 10 tests (struct, bindings, types, compute entry)

**Total:** 41+ tests

---

## Implementation Files

| File | Purpose |
|------|---------|
| `src/shader/ast.rs` | ShaderModule, Type, UniformDecl, StorageDecl |
| `src/shader/parser.rs` | Lexer + parser for .wjsl |
| `src/shader/type_checker.rs` | Host/shader type validation |
| `src/shader/wgsl_codegen.rs` | WGSL output generation |
| `tests/shader_wjsl_test.rs` | Integration tests |

---

## Future Work

1. **Function body parsing** - Parse `@compute fn main()` body from .wjsl
2. **Bounds check insertion** - Automatic bounds checks for buffer access
3. **Struct definition** - Parse struct definitions in .wjsl
4. **Windjammer integration** - `gpu_upload_uniform()` calls validated against .wjsl at compile time

---

## Related Documentation

- `GAME_ENGINE_IMPROVEMENTS_DESIGN.md` - Full design document
- `SCREEN_SIZE_TYPE_MISMATCH_BUG.md` - Bug that motivated this
- `BLACK_SCREEN_POSTMORTEM.md` - Root cause analysis
- `RENDERING_GUARDRAILS_SUMMARY.md` - Runtime validation (complementary)
