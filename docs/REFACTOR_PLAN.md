# Generator.rs Refactoring Plan

**Current State**: 6381 lines, 98 functions in one file  
**Goal**: ~10-15 focused modules, each < 500 lines  
**Method**: TDD - test after each extraction

---

## Module Structure

```
src/codegen/rust/
├── mod.rs                      # Module definitions
├── generator.rs                # Main coordinator (target: <500 lines)
├── literals.rs                 # ✅ DONE (Phase 1)
├── type_casting.rs             # ✅ DONE (Phase 2)
├── framework/
│   ├── mod.rs
│   ├── detection.rs           # Framework detection logic
│   └── game_main.rs           # Game framework main generation
├── expressions/
│   ├── mod.rs
│   ├── binary.rs              # Binary expressions
│   ├── calls.rs               # Function/method calls
│   ├── access.rs              # Field/array access
│   └── special.rs             # Special expressions
├── statements/
│   ├── mod.rs
│   ├── control_flow.rs        # If/match/for/while
│   ├── assignments.rs         # Assignment generation
│   └── declarations.rs        # Let/static
├── items/
│   ├── mod.rs
│   ├── functions.rs           # Function generation
│   ├── structs.rs             # Struct generation
│   ├── enums.rs               # Enum generation
│   ├── traits.rs              # Trait generation
│   └── impls.rs               # Impl block generation
├── inference/
│   ├── mod.rs
│   ├── ownership.rs           # Ownership inference
│   ├── strings.rs             # String type inference
│   └── self_params.rs         # Self parameter inference
├── source_mapping.rs          # Source map tracking
└── helpers.rs                 # Utility functions
```

---

## Phase-by-Phase Plan

### Phase 1: ✅ DONE
- Extracted `literals.rs`

### Phase 2: ✅ DONE  
- Extracted `type_casting.rs`

### Phase 3: Framework Detection (30 min)
- Extract `framework/detection.rs`
- Extract `framework/game_main.rs`
- Functions: `detect_*`, `generate_game_main`

### Phase 4: Source Mapping (20 min)
- Extract `source_mapping.rs`
- Functions: `track_*`, `record_*`, `get_*_location`

### Phase 5: Expression Generation (60 min)
- Extract `expressions/binary.rs`
- Extract `expressions/calls.rs`
- Extract `expressions/access.rs`
- Functions: All expression generation

### Phase 6: Statement Generation (45 min)
- Extract `statements/control_flow.rs`
- Extract `statements/assignments.rs`
- Extract `statements/declarations.rs`
- Functions: All statement generation

### Phase 7: Item Generation (60 min)
- Extract `items/functions.rs`
- Extract `items/structs.rs`
- Extract `items/enums.rs`
- Extract `items/traits.rs`
- Extract `items/impls.rs`
- Functions: All item generation

### Phase 8: Inference Modules (45 min)
- Extract `inference/ownership.rs`
- Extract `inference/strings.rs`
- Extract `inference/self_params.rs`
- Functions: All inference logic

### Phase 9: Helpers & Cleanup (30 min)
- Extract remaining utilities
- Clean up generator.rs
- Ensure < 500 lines

### Phase 10: Documentation & Tests (30 min)
- Add module-level docs
- Add integration tests
- Verify all tests pass

---

## Total Estimated Time: 5-6 hours

---

## Testing Strategy

After each phase:
1. Run `cargo test --lib`
2. Verify 231+ tests pass
3. Check for regressions
4. Commit progress

---

## Success Criteria

- ✅ All modules < 500 lines
- ✅ Clear separation of concerns
- ✅ All 231+ tests passing
- ✅ No regressions
- ✅ Better code organization
- ✅ Easier to maintain
- ✅ Faster to compile (parallel)










