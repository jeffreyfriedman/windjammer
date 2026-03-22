# Separation of Concerns Audit

**Date**: 2025-11-28  
**Auditor**: Windjammer Team  
**Scope**: Windjammer compiler codebase  
**Goal**: Identify library-specific code that violates separation of concerns

---

## Executive Summary

**Findings**: 2 major violations found
- ❌ `@game` decorator (game engine specific)
- ❌ `@component` decorator (UI framework specific)

**Impact**: Compiler is tightly coupled to `windjammer-game` and `windjammer-ui`

**Recommendation**: Remove both decorators, implement proper decorator system in v0.40+

---

## Violation 1: @game Decorator

### Location
- `src/codegen/rust/generator.rs` (lines 406-600+)
- `src/analyzer.rs` (game decorator detection)

### What It Does
Generates boilerplate for game development:
- ECS world setup
- Game loop implementation
- Window creation
- Renderer initialization

### Code References
```rust
// Hardcoded imports
output.push_str("    use windjammer_game::*;\n");
output.push_str("    use windjammer_game::ecs::*;\n");
```

### Severity
**HIGH** - Couples compiler to `windjammer-game` library

### Status
✅ **DEPRECATED** - Marked for removal in v0.40+  
✅ **DOCUMENTED** - See DECORATOR_SYSTEM_DESIGN.md  
✅ **ALTERNATIVE** - Use explicit `GameApp` API

---

## Violation 2: @component Decorator

### Location
- `src/ui/codegen_desktop.rs` (entire file, 509 lines)
- `src/ui/codegen_web.rs` (entire file, 134 lines)
- `src/component/codegen.rs` (entire file, 249 lines)
- `src/codegen/rust/generator.rs` (line 795: component detection)

### What It Does
Generates UI component boilerplate:
- Component struct with Signal fields
- Component trait implementation
- Reactive state management
- Event handlers

### Code References

**File**: `src/ui/codegen_desktop.rs`
```rust
fn generate_imports() -> String {
    r#"use windjammer_ui::Signal;
use windjammer_ui::component::Component;
use windjammer_ui::vdom::VNode;
use windjammer_ui::components::button::Button;
use windjammer_ui::components::text::Text;
use windjammer_ui::components::flex::{Flex, FlexDirection};"#
        .to_string()
}
```

**File**: `src/codegen/rust/generator.rs` (line 795)
```rust
if s.decorators.iter().any(|d| d.name == "component") {
    body.push_str(&self.generate_component_impl(s));
    body.push_str("\n\n");
}
```

### Severity
**HIGH** - Couples compiler to `windjammer-ui` library

### Status
❌ **ACTIVE** - Currently in use  
⚠️ **NEEDS REMOVAL** - Should be removed for separation of concerns

---

## Violation 3: UI Framework Detection

### Location
- `src/codegen/rust/generator.rs` (lines 329-345)

### What It Does
Detects `use std::ui::*` imports and generates UI-specific code

### Code
```rust
fn detect_ui_framework(&self, program: &Program) -> UIFrameworkInfo {
    let mut uses_ui = false;
    
    // Check for use std::ui::* or use std::ui
    for item in &program.items {
        if let Item::Use { path, .. } = item {
            let path_str = path.join("::");
            if path_str == "std::ui" || path_str.starts_with("std::ui::") {
                uses_ui = true;
                break;
            }
        }
    }
    
    UIFrameworkInfo { uses_ui }
}
```

### Severity
**MEDIUM** - Detects UI usage but doesn't hardcode library names

### Status
⚠️ **QUESTIONABLE** - Might be OK if `std::ui` is part of stdlib  
🤔 **NEEDS REVIEW** - Is `std::ui` in stdlib or is it `windjammer-ui`?

---

## Other Findings

### ✅ ACCEPTABLE: Platform API Detection

**Location**: `src/codegen/rust/generator.rs` (lines 346-400)

**What It Does**: Detects usage of `std::fs`, `std::process`, `std::net`, etc.

**Verdict**: **ACCEPTABLE** - These are stdlib modules, not external libraries

```rust
fn detect_platform_apis(&self, program: &Program) -> PlatformApis {
    // Checks for std::fs, std::process, std::dialog, etc.
}
```

---

## Recommendations

### Immediate (v0.38.6)
1. ✅ **Deprecate @game decorator** - Done
2. ❌ **Deprecate @component decorator** - TODO
3. ❌ **Add warnings when these decorators are used** - TODO

### Short Term (v0.39)
1. Remove `src/ui/` directory entirely
2. Remove `src/component/` directory entirely
3. Remove decorator detection from `generator.rs`
4. Update documentation to use explicit APIs

### Long Term (v0.40+)
1. Implement proper decorator system
2. Move `@component` to `windjammer-ui` library
3. Move `@game` to `windjammer-game` library
4. Enable library authors to define decorators

---

## Migration Path

### For @component Users

**Before** (current, deprecated):
```windjammer
@component
fn Counter() -> UI {
    let count = signal(0)
    
    button("Click me")
        .on_click(|| count.set(count.get() + 1))
}
```

**After** (explicit API):
```windjammer
struct Counter {
    count: Signal<int>,
}

impl Counter {
    fn new() -> Counter {
        Counter {
            count: Signal::new(0),
        }
    }
}

impl Component for Counter {
    fn render(self) -> VNode {
        button("Click me")
            .on_click(|| self.count.set(self.count.get() + 1))
    }
}
```

**Future** (library-defined decorator):
```windjammer
use windjammer_ui::decorators::component;

@component
struct Counter {
    count: Signal<int>,
}

// Decorator generates Component impl automatically
```

---

## Impact Analysis

### Current State
- **Compiler Size**: ~5,000 lines
- **Library-Specific Code**: ~900 lines (18%)
  - `@game`: ~200 lines
  - `@component`: ~700 lines
- **Coupling**: Tight coupling to 2 libraries

### After Cleanup
- **Compiler Size**: ~4,100 lines (18% reduction)
- **Library-Specific Code**: 0 lines (0%)
- **Coupling**: Zero coupling to external libraries

### Benefits
1. **Cleaner Architecture**: Compiler is general-purpose
2. **Easier Maintenance**: Library changes don't require compiler changes
3. **Better Extensibility**: Libraries can define their own abstractions
4. **Faster Compilation**: Less code to compile
5. **Better Testing**: Compiler tests don't need library dependencies

---

## Action Items

### High Priority (Blocks MVP)
- [ ] None - decorators are deprecated but don't block MVP

### Medium Priority (v0.39)
- [ ] Remove `src/ui/` directory
- [ ] Remove `src/component/` directory  
- [ ] Remove `@component` detection from generator
- [ ] Remove `@game` detection from generator
- [ ] Update tests to not depend on decorators

### Low Priority (v0.40+)
- [ ] Design proper decorator system
- [ ] Implement template-based decorators
- [ ] Implement compile-time functions
- [ ] Migrate `@component` to `windjammer-ui`
- [ ] Migrate `@game` to `windjammer-game`

---

## Violation 4: Signal Type Mapping

### Location
- `src/codegen/rust/types.rs` (lines 15-42)

### What It Does
Hardcodes `Signal` type to map to `windjammer_ui::reactivity::Signal`

### Code
```rust
// Special case: Signal (without type params) -> windjammer_ui::reactivity::Signal
if name == "Signal" {
    "windjammer_ui::reactivity::Signal".to_string()
}

// Special case: Signal<T> -> windjammer_ui::reactivity::Signal<T>
if base == "Signal" {
    format!("windjammer_ui::reactivity::Signal<{}>", rust_args.join(", "))
}
```

### Severity
**HIGH** - Hardcodes `windjammer-ui` type mapping in compiler

### Status
❌ **ACTIVE** - Currently in use  
⚠️ **NEEDS REMOVAL** - Should be removed

### Recommendation
`Signal` should either:
1. Be in stdlib (`std::reactive::Signal`)
2. Be imported explicitly by users
3. Not have special treatment in compiler

---

## Violation 5: WASM Cargo.toml Generation

### Location
- `src/codegen/wasm.rs` (line 363)

### What It Does
Generates `Cargo.toml` with hardcoded `windjammer-ui` dependency

### Code
```rust
windjammer-ui = { path = "../../../crates/windjammer-ui" }
```

### Severity
**HIGH** - Assumes `windjammer-ui` exists and hardcodes path

### Status
❌ **ACTIVE** - Currently in use  
⚠️ **NEEDS REMOVAL** - Should be removed

### Recommendation
Users should specify their own dependencies in their project's `Cargo.toml`

---

## Complete Audit Results

### Files Checked
- [x] `src/codegen/rust/generator.rs` - ❌ 2 violations (@game, @component)
- [x] `src/ui/codegen_web.rs` - ❌ Violation (windjammer-ui imports)
- [x] `src/ui/codegen_desktop.rs` - ❌ Violation (windjammer-ui imports)
- [x] `src/component/codegen.rs` - ❌ Violation (windjammer-ui imports)
- [x] `src/analyzer.rs` - ⚠️ Game decorator detection (deprecated)
- [x] `src/main.rs` - ✅ Clean (just comments/variable names)
- [x] `src/codegen/wasm.rs` - ❌ Violation (hardcoded dependency)
- [x] `src/codegen/rust/types.rs` - ❌ Violation (Signal type mapping)

### Summary
- **Total Violations**: 5 major violations
- **Affected Files**: 6 files
- **Lines of Code**: ~1,400 lines of library-specific code
- **Percentage**: 28% of compiler is library-specific! 🚨

---

## Stdlib vs Compiler

### ✅ ACCEPTABLE in Stdlib
The following are OK in `windjammer/std/`:
- `std::ui` module (if it's a general UI abstraction)
- `std::game` module (if it's a general game abstraction)
- Platform APIs (`std::fs`, `std::net`, etc.)

### ❌ NOT ACCEPTABLE in Compiler
The following should NOT be in `windjammer/src/`:
- Hardcoded imports to `windjammer-ui`
- Hardcoded imports to `windjammer-game`
- Library-specific code generation
- Decorator implementations for specific libraries

---

## Decision Log

### 2025-11-28: Audit Findings
- **Found**: 2 major violations (@game, @component)
- **Decision**: Deprecate both, remove in v0.39
- **Rationale**: Violates separation of concerns
- **Alternative**: Use explicit APIs (GameApp, Component trait)
- **Future**: Proper decorator system in v0.40+

---

## Summary of Violations

| # | Violation | Location | Severity | Lines | Status |
|---|-----------|----------|----------|-------|--------|
| 1 | `@game` decorator | generator.rs | HIGH | ~200 | Deprecated |
| 2 | `@component` decorator | generator.rs, ui/, component/ | HIGH | ~900 | Active |
| 3 | UI framework detection | generator.rs | MEDIUM | ~20 | Active |
| 4 | Signal type mapping | types.rs | HIGH | ~10 | Active |
| 5 | WASM Cargo.toml | wasm.rs | HIGH | ~1 | Active |

**Total**: ~1,130 lines of library-specific code (23% of compiler)

---

## Cleanup Plan

### Phase 1: Deprecation (v0.38.6) ✅
- [x] Deprecate `@game` decorator
- [x] Document decorator system design
- [x] Complete separation of concerns audit

### Phase 2: Removal (v0.39)
- [ ] Remove `src/ui/` directory (~650 lines)
- [ ] Remove `src/component/` directory (~250 lines)
- [ ] Remove `@component` detection from generator
- [ ] Remove `@game` detection from generator
- [ ] Remove Signal type mapping from types.rs
- [ ] Remove windjammer-ui from WASM Cargo.toml generation
- [ ] Update all tests

### Phase 3: Proper Architecture (v0.40+)
- [ ] Implement decorator system
- [ ] Move `@component` to `windjammer-ui`
- [ ] Move `@game` to `windjammer-game`
- [ ] Move `Signal` to stdlib or make it user-imported

---

## Architectural Principles

### ✅ GOOD: General-Purpose Compiler
- Compiles Windjammer → Rust/JS/WASM
- Handles language features (traits, generics, ownership)
- Provides stdlib (std::fs, std::net, etc.)

### ❌ BAD: Library-Specific Code
- Hardcoded imports to specific libraries
- Decorator implementations for frameworks
- Type mappings to external crates

### 🎯 GOAL: Zero Library Coupling
- Compiler knows nothing about `windjammer-ui`
- Compiler knows nothing about `windjammer-game`
- Libraries define their own abstractions
- Users import what they need

---

**Status**: Audit complete, 5 violations identified, cleanup plan defined  
**Next**: Continue with MVP, schedule cleanup for v0.39

