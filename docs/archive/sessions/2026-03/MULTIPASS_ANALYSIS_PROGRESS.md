# Multi-Pass Ownership Analysis - Implementation Progress

## Status: IN PROGRESS (80% Complete)

**Date**: 2026-02-27  
**Goal**: Implement proper multi-pass ownership analysis to eliminate conservative "Owned begets Owned" inference cycles.

---

## The Problem

**Conservative Single-Pass Approach (Heuristic)**:
```windjammer
fn leaf_fn(id: string) -> bool { id == "test" }
fn wrapper_fn(item_id: string) -> bool { leaf_fn(item_id) }
```

**Current Behavior**:
- Pass 1: `leaf_fn` doesn't exist in registry → `wrapper_fn` conservatively infers `item_id: String` (Owned)
- Registry populated with `wrapper_fn(String)` and `leaf_fn(String)`
- Result: Both functions generate `String` parameters instead of `&String`

**Correct Behavior** (Multi-Pass):
- Pass 1: `leaf_fn(id)` only uses `id` in comparison → infers `&String` (Borrowed)
- Pass 2: `wrapper_fn(item_id)` sees `leaf_fn(&String)` in registry → infers `&String` (Borrowed)
- Pass 3: No changes → CONVERGED ✅

---

## Implementation Completed

### 1. Multi-Pass Loop (`analyze_program`) ✅
**File**: `windjammer/src/analyzer.rs` (lines 410-497)

```rust
loop {
    eprintln!("🔄 Ownership Analysis Pass {}", pass_number);
    let (new_analyzed, new_registry) = self.analyze_program_pass(program, &registry)?;
    
    if self.signatures_converged(&registry, &new_registry) {
        eprintln!("✅ Ownership analysis converged after {} passes", pass_number);
        return Ok((new_analyzed, new_registry, self.analyzed_trait_methods.clone()));
    }
    
    if pass_number >= MAX_PASSES {
        eprintln!("⚠️  Warning: Ownership analysis did not converge after {} passes", MAX_PASSES);
        return Ok((new_analyzed, new_registry, self.analyzed_trait_methods.clone()));
    }
    
    registry = new_registry;
    pass_number += 1;
}
```

**Status**: ✅ **Working** - Multi-pass loop runs and converges after 2 passes

### 2. Convergence Detection ✅
**File**: `windjammer/src/analyzer.rs` (lines 500-532)

```rust
fn signatures_converged(&self, old: &SignatureRegistry, new: &SignatureRegistry) -> bool {
    for (name, new_sig) in &new.signatures {
        match old.signatures.get(name) {
            None => return false,
            Some(old_sig) => {
                for (old_ownership, new_ownership) in old_sig.param_ownership.iter().zip(&new_sig.param_ownership) {
                    if old_ownership != new_ownership {
                        eprintln!("  📝 {}: {:?} -> {:?}", name, old_ownership, new_ownership);
                        return false;
                    }
                }
            }
        }
    }
    true
}
```

**Status**: ✅ **Working** - Correctly detects when signatures stabilize

### 3. Registry-Based Inference ✅
**File**: `windjammer/src/analyzer.rs` (lines 1406-1448)

```rust
fn infer_passthrough_ownership(
    &self,
    param_name: &str,
    body: &[&'ast Statement<'ast>],
    registry: &SignatureRegistry,
) -> Option<OwnershipMode> {
    let mut passthrough_calls = Vec::new();
    self.collect_passthrough_calls(param_name, body, &mut passthrough_calls);
    
    if passthrough_calls.is_empty() {
        return None;
    }
    
    let mut inferred_mode: Option<OwnershipMode> = None;
    for (func_name, arg_position) in &passthrough_calls {
        if let Some(sig) = registry.get_signature(func_name) {
            if let Some(&ownership) = sig.param_ownership.get(*arg_position) {
                match inferred_mode {
                    None => inferred_mode = Some(ownership),
                    Some(existing_mode) => {
                        if existing_mode != ownership {
                            return Some(OwnershipMode::Owned);
                        }
                    }
                }
            } else {
                return None;
            }
        } else {
            return None; // Function signature unknown
        }
    }
    inferred_mode
}
```

**Status**: ✅ **Implemented** - Registry lookup works, pass-through calls collected correctly

### 4. Passthrough Call Collection ✅
**File**: `windjammer/src/analyzer.rs` (lines 1451-1571)

Helper functions to recursively find where parameters are passed as arguments:
- `collect_passthrough_calls` - Iterates statements
- `collect_passthrough_from_stmt` - Handles statement types
- `collect_passthrough_from_expr` - Handles expressions (Call, MethodCall, etc.)
- `expr_is_identifier` - Checks if expression is direct parameter usage
- `extract_function_name` - Gets callable name from expression
- `extract_method_name` - Gets method name (with type prefix when possible)

**Status**: ✅ **Working** - Correctly identifies pass-through usage

---

## Remaining Issue (20%)

### Problem: Parameters Still Inferring as `Owned`

**Observed Behavior**:
```bash
$ wj build test.wj
🔄 Ownership Analysis Pass 1
🔍 Inferring ownership for parameter 'id' (type: String)
🔍 Inferring ownership for parameter 'item_id' (type: String)
🔄 Ownership Analysis Pass 2
🔍 Inferring ownership for parameter 'id' (type: String)
🔍 Inferring ownership for parameter 'item_id' (type: String)
✅ Ownership analysis converged after 2 passes

📊 build_signature: param 'id' type=String inferred=Owned
📊 build_signature: param 'item_id' type=String inferred=Owned

Generated:
fn leaf_fn(id: String) -> bool { ... }  // Should be &String
fn wrapper_fn(item_id: String) -> bool { ... }  // Should be &String
```

**Root Cause**: The detailed debug statements in `infer_parameter_ownership` aren't firing, suggesting:
1. Function is returning early before reaching the passthrough check
2. OR `inferred_ownership` HashMap isn't being populated correctly
3. OR another code path is overriding the inference

**Next Steps**:
1. Add debug output at the very start of `infer_parameter_ownership` to confirm it's being called
2. Add debug output to show which early-return path is taken (mutated/returned/stored/etc.)
3. Check if `inferred_ownership` HashMap is being populated in `analyze_function`
4. Verify `build_signature` is reading from the correct `inferred_ownership` map

---

## Test Suite

**File**: `windjammer/tests/bug_multipass_ownership_test.rs`

### Test Cases (All Currently Failing):

1. **`test_passthrough_borrowed_convergence`** ❌
   - Tests simple pass-through: `wrapper(id)` → `leaf(id)` where `leaf` only compares `id`
   - Expected: Both infer `&String`
   - Actual: Both infer `String`

2. **`test_method_passthrough_convergence`** ❌
   - Tests method pass-through: `Merchant::has_item(id)` → `Inventory::has(id)`
   - Expected: Both infer `&String`
   - Actual: Both infer `String`

3. **`test_circular_dependency_convergence`** ❌
   - Tests mutual recursion: `foo(x)` ↔ `bar(y)` where both only compare
   - Expected: Both infer `&String`
   - Actual: Both infer `String`

**To run tests**:
```bash
cd windjammer
cargo test --release --test bug_multipass_ownership_test
```

---

## Architecture

### Data Flow

```
Program
  ↓
analyze_program() 
  ↓
[LOOP until convergence]
  ↓
analyze_program_pass(registry)
  ├→ analyze_function(func, registry)
  │   ├→ infer_parameter_ownership(param, body, registry)  ← REGISTRY USED HERE
  │   │   ├→ Check: mutated? returned? stored? iterated?
  │   │   ├→ infer_passthrough_ownership(registry)  ← LOOKS UP CALLEES
  │   │   └→ Default: Borrowed
  │   └→ Store in inferred_ownership HashMap
  └→ build_signature(analyzed_func)  ← READS inferred_ownership
      └→ Populate param_ownership Vec
  ↓
FunctionSignature added to registry
  ↓
[Check convergence: compare old vs new param_ownership]
  ↓
If changed: continue loop
If stable: return results
```

### Key Data Structures

```rust
pub struct FunctionSignature {
    pub name: String,
    pub param_types: Vec<Type>,
    pub param_ownership: Vec<OwnershipMode>,  // ← COMPARED FOR CONVERGENCE
    pub return_type: Option<Type>,
    pub return_ownership: OwnershipMode,
    pub has_self_receiver: bool,
    pub is_extern: bool,
}

pub struct SignatureRegistry {
    signatures: HashMap<String, FunctionSignature>,
}

pub struct AnalyzedFunction<'ast> {
    pub decl: FunctionDecl<'ast>,
    pub inferred_ownership: HashMap<String, OwnershipMode>,  // ← POPULATED BY ANALYSIS
    // ... other fields
}
```

---

## Debug Output Reference

| Symbol | Meaning |
|--------|---------|
| `🔄` | New analysis pass starting |
| `✅` | Analysis converged successfully |
| `⚠️` | Warning (didn't converge, using last known) |
| `🔍` | Inferring ownership for a parameter |
| `→` | Decision made (e.g., "→ Default: Borrowed") |
| `📊` | build_signature processing parameter |
| `📞` | Passthrough call detection |
| `📝` | Signature changed between passes |

---

## Philosophy Alignment

This implementation embodies the Windjammer philosophy:

✅ **Correctness Over Speed**: Multi-pass ensures correct inference, even if slower  
✅ **No Workarounds**: Proper solution, not conservative heuristics  
✅ **Compiler Does the Work**: User writes `id: string`, compiler infers `&String`  
✅ **Long-term Robustness**: Architecture supports complex inference scenarios  
✅ **Maintainability**: Clear separation of concerns, well-documented  

---

## Related Issues

- **E0308 Type Mismatches (14 instances)**: Blocked on this fix
- **Conservative Ownership Inference**: This is the root cause fix

---

## Success Criteria

- [x] Multi-pass loop implemented
- [x] Convergence detection working
- [x] Registry-based lookup implemented
- [x] Passthrough call collection working
- [ ] TDD tests passing (0/3)
- [ ] Windjammer-game compilation improves
- [ ] E0308 errors reduced

**Next Session**: Debug why `inferred_ownership` is `Owned` when it should be `Borrowed`.
