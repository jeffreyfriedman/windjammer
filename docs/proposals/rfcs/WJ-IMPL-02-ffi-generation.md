# FFI Generation from IDL - Proposal

## Overview

This document proposes an architecture for automatically generating C FFI bindings from our existing IDL, which would then be used by all 12 language SDKs.

## Current State

### Manual FFI (Current)
```rust
// Manually written in crates/windjammer-c-ffi/src/rendering.rs
#[no_mangle]
pub extern "C" fn wj_sprite_new(
    entity: *mut WjEntity,
    texture: *mut WjTexture,
    position: WjVec2,
    size: WjVec2,
    color: WjColor,
) -> WjErrorCode {
    // Manual panic handling
    // Manual error handling
    // Manual null checks
}
```

### IDL Definition (What We Have)
```rust
// In tools/sdk-generator/src/idl.rs
Class {
    name: "Sprite",
    methods: vec![
        Method {
            name: "new",
            params: vec![
                Param { name: "entity", type_: Type::Handle("Entity") },
                Param { name: "texture", type_: Type::Handle("Texture") },
                Param { name: "position", type_: Type::Struct("Vec2") },
                // ...
            ],
            return_type: Type::Result(Box::new(Type::Void)),
        }
    ]
}
```

## Proposed Architecture

### Phase 1: Extend IDL with FFI Metadata

```rust
// Enhanced IDL with FFI-specific metadata
#[derive(Debug, Clone)]
pub struct FfiMetadata {
    /// C function name (e.g., "wj_sprite_new")
    pub c_name: String,
    
    /// Whether this function can panic
    pub can_panic: bool,
    
    /// Custom error handling strategy
    pub error_strategy: ErrorStrategy,
    
    /// Memory ownership rules
    pub ownership: OwnershipRules,
}

#[derive(Debug, Clone)]
pub enum ErrorStrategy {
    /// Return error code
    ReturnCode,
    /// Set last error + return null
    SetLastError,
    /// Custom error handling
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct OwnershipRules {
    /// Who owns returned pointers
    pub return_ownership: Ownership,
    /// Who owns input parameters
    pub param_ownership: Vec<Ownership>,
}

#[derive(Debug, Clone)]
pub enum Ownership {
    /// Caller owns (must free)
    Caller,
    /// Callee owns (don't free)
    Callee,
    /// Borrowed (temporary)
    Borrowed,
}
```

### Phase 2: FFI Code Generator

```rust
// tools/sdk-generator/src/ffi_generator.rs

pub struct FfiGenerator {
    idl: ApiDefinition,
}

impl FfiGenerator {
    pub fn generate_c_ffi(&self) -> String {
        let mut output = String::new();
        
        // Generate header
        output.push_str(&self.generate_header());
        
        // Generate functions for each class/method
        for class in &self.idl.classes {
            for method in &class.methods {
                output.push_str(&self.generate_ffi_function(class, method));
            }
        }
        
        output
    }
    
    fn generate_ffi_function(&self, class: &Class, method: &Method) -> String {
        let c_name = format!("wj_{}_{}", 
            class.name.to_lowercase(), 
            method.name
        );
        
        let params = self.generate_params(method);
        let return_type = self.generate_return_type(method);
        let body = self.generate_body(class, method);
        
        format!(r#"
/// {doc}
#[no_mangle]
pub extern "C" fn {c_name}({params}) -> {return_type} {{
{body}
}}
"#,
            doc = method.documentation.as_deref().unwrap_or(""),
            c_name = c_name,
            params = params,
            return_type = return_type,
            body = body,
        )
    }
    
    fn generate_body(&self, class: &Class, method: &Method) -> String {
        let mut body = String::new();
        
        // 1. Null pointer checks
        body.push_str(&self.generate_null_checks(method));
        
        // 2. Panic handling wrapper
        body.push_str("    let result = panic::catch_unwind(|| {\n");
        
        // 3. Type conversions (C types -> Rust types)
        body.push_str(&self.generate_type_conversions(method));
        
        // 4. Call actual Rust implementation
        body.push_str(&self.generate_rust_call(class, method));
        
        // 5. Convert result back to C types
        body.push_str(&self.generate_result_conversion(method));
        
        body.push_str("    });\n");
        
        // 6. Handle panic/error
        body.push_str(&self.generate_error_handling(method));
        
        body
    }
}
```

### Phase 3: Integration with Build System

```toml
# Cargo.toml for windjammer-c-ffi
[build-dependencies]
windjammer-sdk-generator = { path = "../tools/sdk-generator" }

# build.rs
fn main() {
    // Load IDL
    let idl = load_idl("../../idl/api.json");
    
    // Generate FFI bindings
    let ffi_code = FfiGenerator::new(idl).generate_c_ffi();
    
    // Write to src/generated/
    std::fs::write("src/generated/ffi.rs", ffi_code)?;
    
    // Generate C headers
    let c_headers = FfiGenerator::new(idl).generate_c_headers();
    std::fs::write("include/windjammer_generated.h", c_headers)?;
}
```

## Example: Generated vs Manual

### IDL Definition

```json
{
  "class": "Sprite",
  "methods": [
    {
      "name": "new",
      "params": [
        {"name": "entity", "type": "Handle<Entity>"},
        {"name": "texture", "type": "Handle<Texture>"},
        {"name": "position", "type": "Vec2"},
        {"name": "size", "type": "Vec2"},
        {"name": "color", "type": "Color"}
      ],
      "return_type": "Result<void>",
      "ffi_metadata": {
        "can_panic": true,
        "error_strategy": "ReturnCode",
        "ownership": {
          "return_ownership": "Caller",
          "param_ownership": ["Borrowed", "Borrowed", "Borrowed", "Borrowed", "Borrowed"]
        }
      }
    }
  ]
}
```

### Generated FFI Code

```rust
// AUTO-GENERATED - DO NOT EDIT
// Generated from IDL by windjammer-sdk-generator

/// Create a new sprite
#[no_mangle]
pub extern "C" fn wj_sprite_new(
    entity: *mut WjEntity,
    texture: *mut WjTexture,
    position: WjVec2,
    size: WjVec2,
    color: WjColor,
) -> WjErrorCode {
    // Null pointer checks (generated)
    if entity.is_null() {
        set_last_error("Null entity pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    if texture.is_null() {
        set_last_error("Null texture pointer".to_string());
        return WjErrorCode::NullPointer;
    }
    
    // Panic handling (generated)
    let result = panic::catch_unwind(|| {
        // Type conversions (generated)
        let entity_handle = unsafe { EntityHandle::from_raw(entity) };
        let texture_handle = unsafe { TextureHandle::from_raw(texture) };
        let pos: glam::Vec2 = position.into();
        let sz: glam::Vec2 = size.into();
        let col: Color = color.into();
        
        // Call Rust implementation (generated)
        windjammer_game_framework::rendering::sprite_new(
            entity_handle,
            texture_handle,
            pos,
            sz,
            col,
        )
        .map(|_| WjErrorCode::Ok)
        .unwrap_or_else(|e| {
            set_last_error(e.to_string());
            WjErrorCode::OperationFailed
        })
    });
    
    // Error handling (generated)
    match result {
        Ok(code) => code,
        Err(e) => {
            set_last_error(format!("Panic in wj_sprite_new: {:?}", e));
            WjErrorCode::Panic
        }
    }
}
```

## Benefits

### 1. Consistency
- All FFI functions follow the same patterns
- No manual errors or inconsistencies
- Guaranteed null checks and panic handling

### 2. Maintainability
- Update IDL once, regenerate everything
- Easy to add new functions
- Refactoring is automatic

### 3. Documentation
- Auto-generate C headers with docs
- Auto-generate SDK documentation
- Single source of truth

### 4. Type Safety
- Compile-time guarantees
- Ownership rules enforced
- Memory safety verified

### 5. Evolution
```
Add feature to IDL
      ↓
Regenerate FFI (automatic)
      ↓
Regenerate SDKs (automatic)
      ↓
Update examples (semi-automatic)
      ↓
Run tests (automatic)
      ↓
Ship to users
```

## Migration Path

### Phase 1: Proof of Concept (1-2 weeks)
- [ ] Extend IDL with FFI metadata
- [ ] Create FFI generator
- [ ] Generate one module (e.g., rendering.rs)
- [ ] Compare with manual version
- [ ] Verify it compiles and tests pass

### Phase 2: Gradual Migration (2-3 weeks)
- [ ] Generate all core modules
- [ ] Keep manual versions as fallback
- [ ] Test generated vs manual
- [ ] Fix any issues

### Phase 3: Full Adoption (1 week)
- [ ] Remove manual FFI code
- [ ] Use only generated code
- [ ] Update build system
- [ ] Update documentation

### Phase 4: SDK Integration (2-3 weeks)
- [ ] Update SDK generators to use FFI metadata
- [ ] Regenerate all 12 SDKs
- [ ] Test all examples
- [ ] Verify performance

## Challenges

### 1. Complex Error Handling
Some functions need custom error handling that's hard to express in IDL.

**Solution**: Allow custom error handlers in IDL or fallback to manual.

### 2. Performance
Generated code might not be as optimized as hand-written.

**Solution**: Profile and optimize generator, or allow manual overrides.

### 3. Edge Cases
Some functions have special requirements (callbacks, lifetimes, etc.).

**Solution**: Support escape hatches for manual code when needed.

### 4. Build Complexity
Code generation adds complexity to build process.

**Solution**: Cache generated code, only regenerate when IDL changes.

## Recommendation

**YES, we should move to IDL-based FFI generation!**

### Immediate Actions (This Week)
1. ✅ Keep current manual FFI (it's working)
2. 🔄 Extend IDL with FFI metadata
3. 🔄 Create proof-of-concept generator
4. 🔄 Generate one module and compare

### Short-term (Next Month)
1. Generate all FFI modules
2. Test thoroughly
3. Migrate gradually
4. Keep manual as fallback

### Long-term (3-6 Months)
1. Full IDL-based generation
2. Auto-regenerate on IDL changes
3. CI/CD integration
4. Documentation generation

## Conclusion

**IDL-based FFI generation is the right long-term solution.**

It provides:
- ✅ Consistency across all bindings
- ✅ Easier maintenance
- ✅ Faster feature development
- ✅ Better documentation
- ✅ Type safety guarantees

The current manual approach is fine for now (105 functions is manageable), but as we scale to 200+ functions, IDL-based generation becomes essential.

**Recommendation**: Continue manual FFI for Phase 4 (next ~95 functions), then migrate to IDL-based generation before scaling further.

---

*This proposal ensures Windjammer can scale efficiently while maintaining quality and consistency across all 12 language SDKs.*

