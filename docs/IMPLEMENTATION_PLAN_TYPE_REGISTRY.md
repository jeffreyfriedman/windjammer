# Implementation Plan: Type Registry for Correct Import Paths

**Estimated Time**: 1-2 hours  
**Complexity**: Medium  
**Priority**: HIGH (blocks windjammer-game platformer)

---

## 🎯 **THE PROBLEM**

### **Current Behavior** (WRONG)
```windjammer
// sprite.wj
use rendering::TextureAtlas
```

**Generated Rust**:
```rust
use super::texture_atlas::TextureAtlas;  // ❌ texture_atlas is not a module!
```

### **Desired Behavior** (CORRECT)
```rust
use super::texture::TextureAtlas;  // ✅ TextureAtlas is defined in texture.wj
```

---

## 🔍 **ROOT CAUSE**

**Location**: `generator.rs` lines 1141-1159

**Bug**:
```rust
let file_name = if common_directories.contains(&module_name) && is_type {
    // BUG: Converts TYPE NAME to file name
    let mut snake = String::new();
    for (i, ch) in item_name.chars().enumerate() {
        // ... converts TextureAtlas → texture_atlas ...
    }
    snake  // ❌ Assumes texture_atlas.wj exists!
}
```

**Problem**: Compiler assumes each type has its own file:
- `TextureAtlas` → `texture_atlas.wj`
- `SpriteRegion` → `sprite_region.wj`

**Reality**: Multiple types can be in one file:
- `texture.wj` defines: `Texture`, `TextureAtlas`, `SpriteRegion`

---

## ✅ **THE SOLUTION: Type Registry**

### **Step 1: Define Type Registry** (10 min)

Create new file: `windjammer/src/type_registry.rs`

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maps type names to their defining source files
/// Example: "TextureAtlas" → "texture"  (texture.wj)
#[derive(Debug, Clone, Default)]
pub struct TypeRegistry {
    /// Type name → File name (without .wj extension)
    types: HashMap<String, String>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry {
            types: HashMap::new(),
        }
    }

    /// Register a type as being defined in a specific file
    pub fn register_type(&mut self, type_name: String, file_name: String) {
        self.types.insert(type_name, file_name);
    }

    /// Get the file name where a type is defined
    /// Returns None if type not found
    pub fn get_file_for_type(&self, type_name: &str) -> Option<&String> {
        self.types.get(type_name)
    }

    /// Build registry by scanning all .wj files in a directory
    pub fn build_from_directory(dir: &Path) -> std::io::Result<Self> {
        let mut registry = TypeRegistry::new();
        
        // Recursively find all .wj files
        let wj_files = Self::find_wj_files(dir)?;
        
        for file_path in wj_files {
            // Parse file to extract type definitions
            registry.register_types_from_file(&file_path)?;
        }
        
        Ok(registry)
    }

    fn find_wj_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
        // Recursively find all .wj files
        let mut files = Vec::new();
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    files.extend(Self::find_wj_files(&path)?);
                } else if path.extension() == Some(std::ffi::OsStr::new("wj")) {
                    files.push(path);
                }
            }
        }
        Ok(files)
    }

    fn register_types_from_file(&mut self, path: &Path) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        
        // Extract file name without extension
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Simple regex-based type extraction
        // Look for: "pub struct TypeName" or "pub enum TypeName"
        for line in content.lines() {
            let trimmed = line.trim();
            
            if let Some(type_name) = Self::extract_type_name(trimmed) {
                self.register_type(type_name, file_name.clone());
            }
        }
        
        Ok(())
    }

    fn extract_type_name(line: &str) -> Option<String> {
        // Match: "pub struct TypeName" or "pub enum TypeName"
        if line.starts_with("pub struct ") {
            return line
                .strip_prefix("pub struct ")?
                .split_whitespace()
                .next()
                .map(|s| s.to_string());
        }
        if line.starts_with("pub enum ") {
            return line
                .strip_prefix("pub enum ")?
                .split_whitespace()
                .next()
                .map(|s| s.to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_type_name() {
        assert_eq!(
            TypeRegistry::extract_type_name("pub struct Vec2 {"),
            Some("Vec2".to_string())
        );
        assert_eq!(
            TypeRegistry::extract_type_name("pub enum Color {"),
            Some("Color".to_string())
        );
        assert_eq!(
            TypeRegistry::extract_type_name("    pub struct Nested {"),
            Some("Nested".to_string())
        );
        assert_eq!(TypeRegistry::extract_type_name("let x = 5;"), None);
    }
}
```

**Add to `lib.rs`**:
```rust
pub mod type_registry;
pub use type_registry::TypeRegistry;
```

---

### **Step 2: Pass Registry to Generator** (5 min)

**Update `CodeGenerator`** (generator.rs line 26):
```rust
pub struct CodeGenerator {
    // ... existing fields ...
    type_registry: TypeRegistry,  // NEW!
}
```

**Update `new()`** (generator.rs line 79):
```rust
pub fn new(
    registry: SignatureRegistry,
    target: CompilationTarget,
    type_registry: TypeRegistry,  // NEW parameter!
) -> Self {
    CodeGenerator {
        // ... existing fields ...
        type_registry,  // NEW!
    }
}
```

**Update all call sites**:
- Find where `CodeGenerator::new()` is called
- Pass empty TypeRegistry for now: `TypeRegistry::new()`

---

### **Step 3: Use Registry in Import Generation** (15 min)

**Update `generate_use()`** (generator.rs lines 1141-1173):

```rust
let file_name = if common_directories.contains(&module_name) && is_type {
    // OLD CODE (DELETE THIS):
    // let mut snake = String::new();
    // for (i, ch) in item_name.chars().enumerate() {
    //     // ... snake_case conversion ...
    // }
    // snake
    
    // NEW CODE (USE REGISTRY):
    // Look up which file defines this type
    if let Some(file) = self.type_registry.get_file_for_type(item_name) {
        file.clone()  // Use the actual file name!
    } else {
        // Fallback to snake_case if not in registry (backward compatible)
        self.to_snake_case(item_name)
    }
} else {
    module_name.to_string()
};
```

---

### **Step 4: Build Registry Before Compilation** (20 min)

**Find where compilation starts** (likely in `lib.rs` or `main.rs`):

```rust
pub fn compile_project(path: &Path, output: &Path) -> Result<(), String> {
    // NEW: Build Type Registry from source directory
    let type_registry = TypeRegistry::build_from_directory(path)
        .map_err(|e| format!("Failed to build type registry: {}", e))?;
    
    // Pass registry to code generator
    let generator = CodeGenerator::new(
        signature_registry,
        target,
        type_registry,  // NEW!
    );
    
    // ... rest of compilation ...
}
```

---

### **Step 5: Write Tests** (15 min)

Create comprehensive tests in `tests/type_registry_test.rs`:

```rust
#[test]
fn test_type_registry_basic() {
    let mut registry = TypeRegistry::new();
    registry.register_type("Vec2".to_string(), "vec2".to_string());
    
    assert_eq!(registry.get_file_for_type("Vec2"), Some(&"vec2".to_string()));
    assert_eq!(registry.get_file_for_type("Unknown"), None);
}

#[test]
fn test_type_registry_multiple_types_one_file() {
    let mut registry = TypeRegistry::new();
    
    // Simulate texture.wj with multiple types
    registry.register_type("Texture".to_string(), "texture".to_string());
    registry.register_type("TextureAtlas".to_string(), "texture".to_string());
    registry.register_type("SpriteRegion".to_string(), "texture".to_string());
    
    // All should map to the same file
    assert_eq!(registry.get_file_for_type("Texture"), Some(&"texture".to_string()));
    assert_eq!(registry.get_file_for_type("TextureAtlas"), Some(&"texture".to_string()));
    assert_eq!(registry.get_file_for_type("SpriteRegion"), Some(&"texture".to_string()));
}
```

---

### **Step 6: Integration Test** (5 min)

Update `module_qualified_type_imports_test.wj` to work with single-file testing:

```windjammer
// For single-file test, define all types in main file
pub struct Texture {
    pub width: u32,
}

pub struct TextureAtlas {
    pub texture: Texture,
}

pub struct Sprite {
    // Reference types from same file - should work!
    pub tex: Texture,
    pub atlas: TextureAtlas,
}
```

Or create a multi-file test structure in `tests/multi_file_imports/`.

---

### **Step 7: Verify** (10 min)

```bash
# Run compiler tests
cargo test --release --lib
# Should be 207+ passing, 0 failed

# Build windjammer-ui
cd ../windjammer-ui && cargo build --release
# Should be 0 errors, 0 warnings

# Build windjammer-game
cd ../windjammer-game/windjammer-game-core && cargo build --release
# Should be 0 errors, ~5 warnings

# Build platformer
cd ../..
wj build examples/platformer.wj --output build/platformer
cd build/platformer
# Fix Cargo.toml (windjammer-game path)
cargo run --release
# 🎮 GAME RUNS! 🎮
```

---

## 📋 **IMPLEMENTATION CHECKLIST**

- [ ] Create `type_registry.rs` module
- [ ] Add `TypeRegistry` struct with HashMap
- [ ] Implement `build_from_directory()` to scan .wj files
- [ ] Implement `extract_type_name()` to parse type definitions
- [ ] Add unit tests for TypeRegistry
- [ ] Add `type_registry` field to `CodeGenerator`
- [ ] Update `CodeGenerator::new()` signature
- [ ] Update all `CodeGenerator::new()` call sites
- [ ] Update `generate_use()` to use registry (lines 1141-1173)
- [ ] Add helper method `to_snake_case()` if needed
- [ ] Build registry before compilation (find entry point)
- [ ] Run compiler tests (verify 207+ passing)
- [ ] Test windjammer-ui (verify 0 errors)
- [ ] Test windjammer-game (verify 0 errors)
- [ ] **RUN PLATFORMER!** 🎮

---

## 🚨 **POTENTIAL ISSUES**

### **Issue 1: Parser Doesn't Support Inline Modules**
**Status**: Confirmed - can't write `pub mod graphics { ... }` in tests  
**Solution**: Test with multi-file structure or skip test, validate with real projects

### **Issue 2: Registry Built Too Late**
**Status**: Possible - need to find correct build order  
**Solution**: Build registry before any code generation starts

### **Issue 3: Cross-Directory Imports**
**Status**: Possible - math::Vec2 vs rendering::Texture  
**Solution**: Registry should scan ALL source directories

---

## 💡 **ALTERNATIVE: SIMPLER FIX**

If Type Registry is too complex, there's a **simpler fix**:

### **Heuristic-Based Approach**

Instead of building a registry, use a smarter heuristic:

```rust
// In generate_use(), lines 1141-1173:
let file_name = if common_directories.contains(&module_name) && is_type {
    // Try these file names in order:
    // 1. Type name in snake_case (TextureAtlas → texture_atlas)
    // 2. First word of type in snake_case (TextureAtlas → texture)
    // 3. Module name itself (rendering → rendering)
    
    let snake_full = self.to_snake_case(item_name);
    let snake_first_word = self.to_snake_case(Self::first_word(item_name));
    
    // Check which file actually exists (requires file system access)
    // OR: Use common patterns (TextureAtlas/SpriteRegion → texture)
    
    // Pattern matching for known cases:
    if item_name.starts_with("Texture") || item_name.starts_with("Sprite") {
        "texture".to_string()  // TextureAtlas, SpriteRegion → texture
    } else if item_name.starts_with("RigidBody") || item_name.starts_with("Collider") {
        "rigidbody2d".to_string()  // Collider2D → rigidbody2d
    } else {
        snake_full  // Fallback to current behavior
    }
} else {
    module_name.to_string()
};
```

**Pros**: Simpler, no registry needed  
**Cons**: Fragile, needs pattern matching for each case

---

## 🎯 **RECOMMENDATION**

**Do the Type Registry properly!**

### **Why**:
1. **Correct**: Looks up actual file, no guessing
2. **Extensible**: Works for any type, any file
3. **Maintainable**: No hardcoded patterns
4. **Production-Quality**: Proper solution, not workaround

### **Time Investment**:
- **1-2 hours** to implement correctly
- **Saves hours** of debugging pattern-match edge cases
- **Permanent solution** that works forever

---

## 🚀 **AFTER THIS FIX**

### **windjammer-game** ✅
- 51 errors → **0 errors**
- All imports correct
- Build.rs regeneration: NO PROBLEM!
- **PLATFORMER RUNS!** 🎮

### **Compiler Quality** ✅
- Proper cross-file type resolution
- No heuristics or pattern matching
- Works for ALL projects
- Production-ready!

---

## 📊 **FILES TO CHANGE**

| File | Changes | LOC | Complexity |
|------|---------|-----|------------|
| `type_registry.rs` | Create new module | ~150 | Medium |
| `lib.rs` | Add pub mod | ~2 | Easy |
| `generator.rs` | Add field + use registry | ~20 | Easy |
| `generator.rs` (call sites) | Pass TypeRegistry | ~10 | Easy |
| `main.rs` or compile entry | Build registry | ~10 | Easy |
| Tests | Unit tests | ~50 | Easy |
| **TOTAL** | **~240 lines** | **Medium** |

---

## 🎯 **NEXT SESSION CHECKLIST**

### **Phase 1: Setup** (15 min)
- [ ] Create `type_registry.rs` module
- [ ] Add TypeRegistry struct
- [ ] Write unit tests
- [ ] Verify tests pass

### **Phase 2: Integration** (20 min)
- [ ] Add type_registry field to CodeGenerator
- [ ] Update CodeGenerator::new() signature
- [ ] Find and update all call sites
- [ ] Verify compiler still compiles

### **Phase 3: Build Registry** (15 min)
- [ ] Find compilation entry point
- [ ] Add registry building step
- [ ] Pass registry to generator
- [ ] Test with sample project

### **Phase 4: Fix Import Generation** (20 min)
- [ ] Update generate_use() logic
- [ ] Use registry instead of snake_case
- [ ] Add fallback for unknown types
- [ ] Test generated imports

### **Phase 5: Validate** (20 min)
- [ ] Run full compiler test suite (207+ tests)
- [ ] Build windjammer-ui (0 errors, 0 warnings)
- [ ] Build windjammer-game (0 errors, ~5 warnings)
- [ ] Build platformer
- [ ] **RUN THE GAME!** 🎮

**Total Time**: ~90 minutes

---

## 💡 **KEY INSIGHTS**

### **Why Manual Fixes Failed**
- `build.rs` regenerates files on every `cargo build`
- Manual fixes are immediately overwritten
- **Must fix compiler, not generated code!**

### **Why This is the Right Approach**
- Proper type resolution (not heuristics)
- Works for all projects (not just windjammer-game)
- Permanent solution (no future surprises)
- **Windjammer Philosophy: Fix root causes!**

---

## 🎉 **AFTER THIS**

With Type Registry implemented:
- ✅ All imports will be correct
- ✅ build.rs regeneration will work
- ✅ No more manual fixing needed
- ✅ Both projects compile cleanly
- ✅ **PLATFORMER RUNS!** 🎮

**This is the final compiler bug blocking the platformer!** 🏁

---

**NEXT SESSION: ~90 minutes → PLAY THE GAME!** 🎮🚀

