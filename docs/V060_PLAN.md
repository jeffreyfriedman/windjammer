# v0.6.0 Development Plan

## Overview

Building on the successful v0.5.0 module system, v0.6.0 will extend modules to support **user-defined modules** and add **generics** for more flexible code.

---

## Goals

### Primary Goals
1. **User-Defined Modules** - Allow developers to create their own modules
2. **Relative Imports** - Import local modules with `use ./my_module`
3. **Module Aliases** - `use std.fs as filesystem`
4. **Basic Generics** - Generic functions and structs

### Secondary Goals
5. **Selective Imports** - `use std.fs.{read, write}`
6. **Re-exports** - `pub use` for module composition
7. **Better Error Messages** - Map Rust errors back to Windjammer source

---

## Feature Details

### 1. User-Defined Modules

**Current**: Only `std.*` modules work
**Target**: Users can create their own modules

**Design**:
```windjammer
// src/math/geometry.wj
pub fn area_circle(radius: f64) -> f64 {
    3.14159 * radius * radius
}

pub fn area_rectangle(width: f64, height: f64) -> f64 {
    width * height
}
```

```windjammer
// src/main.wj
use ./math/geometry

fn main() {
    let area = geometry.area_circle(5.0)
    println!("Area: {}", area)
}
```

**Implementation**:
- Extend module resolution to handle relative paths
- Support `.wj` file discovery
- Handle directory-based modules (`./math` → `math/mod.wj` or `math.wj`)
- Track module dependencies to prevent cycles

### 2. Relative Imports

**Syntax**:
```windjammer
use ./module_name        // Same directory
use ../sibling_module    // Parent directory
use ./utils/helpers      // Subdirectory
```

**Implementation**:
- Parse relative paths in `use` statements
- Resolve paths relative to current file
- Support both file and directory modules

### 3. Module Aliases

**Syntax**:
```windjammer
use std.fs as filesystem
use std.json as j

fn main() {
    filesystem.read_to_string("file.txt")
    j.parse("{}")
}
```

**Implementation**:
- Extend `use` statement parser for `as` keyword
- Track aliases in module resolution
- Update codegen to use aliased names

### 4. Basic Generics

**Syntax**:
```windjammer
// Generic function
fn identity<T>(value: T) -> T {
    value
}

// Generic struct
struct Box<T> {
    value: T,
}

impl<T> Box<T> {
    fn new(value: T) -> Box<T> {
        Box { value }
    }
    
    fn get(&self) -> &T {
        &self.value
    }
}

// Generic enum
enum Option<T> {
    Some(T),
    None,
}
```

**Implementation**:
- Add type parameter parsing (`<T>`, `<T, U>`)
- Extend type system to track generic parameters
- Update codegen to preserve generics in Rust output
- Type inference for generic function calls

### 5. Selective Imports

**Syntax**:
```windjammer
use std.fs.{read, write, exists}

fn main() {
    read("file.txt")  // No fs. prefix needed
    write("out.txt", "data")
    exists("/tmp")
}
```

**Implementation**:
- Parse import lists `{name1, name2}`
- Import specific items into scope
- Update symbol resolution

### 6. Re-exports

**Syntax**:
```windjammer
// utils/mod.wj
pub use ./strings
pub use ./math

// main.wj
use ./utils  // Gets both strings and math
```

**Implementation**:
- Add `pub use` parsing
- Track re-exported symbols
- Resolve transitive exports

---

## Implementation Strategy

### Phase 1: User-Defined Modules (Week 1)
1. ✅ Design module resolution algorithm
2. Extend parser for relative paths
3. Update ModuleCompiler for user modules
4. Add file/directory module support
5. Test with simple examples

### Phase 2: Module Aliases (Week 1)
1. Add `as` keyword to lexer
2. Parse `use X as Y` syntax
3. Track aliases in symbol table
4. Update codegen for aliased names
5. Test aliasing

### Phase 3: Basic Generics (Week 2)
1. Add generic parameter parsing
2. Extend type system for generics
3. Update struct/enum/function parsing
4. Implement type parameter tracking
5. Update codegen to preserve generics
6. Add type inference basics

### Phase 4: Selective Imports (Week 2)
1. Parse import lists `{a, b, c}`
2. Update symbol resolution
3. Test selective imports

### Phase 5: Re-exports (Week 3)
1. Add `pub use` parsing
2. Track re-exported symbols
3. Resolve transitive exports

### Phase 6: Testing & Documentation (Week 3)
1. Comprehensive test suite
2. Update MODULE_SYSTEM.md
3. Create examples for all features
4. Update README

---

## Test Plan

### User Module Tests
- Single file module
- Directory module (mod.wj)
- Nested modules
- Circular dependency detection

### Alias Tests
- Simple alias (`as name`)
- Multiple aliases
- Aliased module usage

### Generic Tests
- Generic function
- Generic struct
- Generic enum
- Multiple type parameters
- Type inference

### Selective Import Tests
- Single import
- Multiple imports
- Wildcard with selective

### Integration Tests
- User modules + aliases
- Generics + modules
- Full feature combination

---

## Examples to Create

### Example 1: Simple User Module
```
src/
  main.wj
  utils.wj
```

### Example 2: Multi-File Project
```
src/
  main.wj
  math/
    mod.wj
    geometry.wj
    algebra.wj
```

### Example 3: Generic Data Structures
```
src/
  main.wj
  collections/
    stack.wj
    queue.wj
```

---

## Success Criteria

✅ User can create and import their own modules
✅ Relative paths work (`./module`)
✅ Module aliases work (`use X as Y`)
✅ Basic generics compile correctly
✅ All existing examples still work
✅ Comprehensive documentation

---

## Risks & Mitigations

### Risk 1: Circular Dependencies
**Mitigation**: Detect cycles during compilation, fail with clear error

### Risk 2: Path Resolution Complexity
**Mitigation**: Start simple (single file), expand gradually

### Risk 3: Generics Complexity
**Mitigation**: Start with basic monomorphization, defer advanced features

### Risk 4: Breaking Changes
**Mitigation**: Ensure backward compatibility, all v0.5.0 code works

---

## Documentation Updates

- `docs/MODULE_SYSTEM.md` - Add user module section
- `docs/GENERICS.md` - New document for generics
- `README.md` - Update features list
- `GUIDE.md` - Add examples for new features

---

## Timeline

**Total Estimated Time**: 3 weeks

- Week 1: User modules + aliases
- Week 2: Generics + selective imports  
- Week 3: Re-exports + testing + docs

**Target Release**: Late October 2025

---

## Next Steps

1. Start with user-defined modules (simplest)
2. Get one example working end-to-end
3. Add aliases (small feature)
4. Move to generics (bigger feature)
5. Polish and document

Let's build v0.6.0! 🚀
