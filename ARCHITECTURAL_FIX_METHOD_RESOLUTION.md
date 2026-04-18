# Architectural Fix: Replace Heuristics with Proper Type-Based Method Resolution

## Current Problem

The `method_call_analyzer.rs` file contains **47+ instances** of hard-coded string matching heuristics:

### Categories of Heuristics Found

1. **Method name matching** (15+ instances):
   ```rust
   matches!(method, "push" | "insert" | "append" | "extend" | ...)
   matches!(method, "has_item" | "get_item" | "remove_item" | ...)
   matches!(method, "contains_key" | "get" | "get_mut" | ...)
   method.starts_with("add_") || method.starts_with("set_") || ...
   method.ends_with("_new")
   ```

2. **Argument name matching** (8+ instances):
   ```rust
   arg_name.ends_with("_id") || arg_name.ends_with("_name") || ...
   matches!(arg_name.as_str(), "id" | "name" | "key" | "item_id" | ...)
   matches!(name.as_str(), "i" | "j" | "k" | "idx" | "pos" | ...)
   ```

3. **Field name matching** (3+ instances):
   ```rust
   matches!(field.as_str(), "id" | "idx" | "index" | "count" | "x" | "y" | "z" | ...)
   ```

4. **Type name matching** (5+ instances):
   ```rust
   matches!(base, "HashMap" | "BTreeMap" | "IndexMap")
   matches!(name.as_str(), "i8" | "i16" | "i32" | ...)
   ```

5. **Game/UI-specific heuristics**:
   ```rust
   // These don't belong in a general-purpose compiler!
   ("draw_text", 0) => true
   ("set_title", 0) => true
   ("log", 0) => true
   "has_item" | "get_attribute" | "is_quest_complete"
   ```

### Why This Is Wrong

1. **Not general-purpose**: Game-specific methods (`has_item`, `get_attribute`) in compiler
2. **Brittle**: Breaks with different naming conventions
3. **Incomplete**: Can't handle all possible method names
4. **Error-prone**: False positives/negatives based on naming
5. **Unmaintainable**: Every new pattern requires code changes
6. **Against Windjammer philosophy**: Compiler should understand code structure, not guess based on names

## The Proper Solution: Type-Based Analysis

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ 1. SIGNATURE COLLECTION PHASE (First Pass)                  │
│    - Collect ALL function/method signatures                 │
│    - Build complete type registry                           │
│    - Store field types for all structs                      │
│    - Track method receivers and parameter types             │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. TYPE RESOLUTION PHASE                                    │
│    - For `obj.field.method(arg)`:                           │
│      1. Resolve `obj` type                                  │
│      2. Look up `field` type from struct definition         │
│      3. Find `method` on field's type                       │
│      4. Get actual parameter types                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. PARAMETER MATCHING (Code Generation)                     │
│    - Check ACTUAL parameter type vs argument type           │
│    - Add `&` only when:                                     │
│      • Param is `&str` AND arg is `String`                  │
│      • Param is `&T` AND arg is `T` (non-Copy)              │
│    - Clone only when:                                       │
│      • Param is `T` AND arg is `&T` (non-Copy)              │
└─────────────────────────────────────────────────────────────┘
```

### Implementation Steps

#### Step 1: Enhanced Type Registry

```rust
pub struct TypeRegistry {
    // Existing
    pub struct_field_types: HashMap<String, HashMap<String, Type>>,
    pub global_signatures: HashMap<String, MethodSignature>,
    
    // NEW: Track method signatures by receiver type
    pub method_signatures_by_type: HashMap<String, HashMap<String, MethodSignature>>,
    // e.g., "Inventory" -> { "has_item" -> MethodSignature { params: [("item_id", &str), ("qty", i32)], ... } }
    
    // NEW: Track imported types and their methods (for stdlib/external crates)
    pub external_method_signatures: HashMap<String, HashMap<String, MethodSignature>>,
}
```

#### Step 2: Field-Chain Type Resolution

```rust
impl TypeAnalyzer {
    /// Resolve the type of a field access chain
    /// Example: `game_state.inventory.has_item` 
    ///   1. game_state: GameState (or &GameState)
    ///   2. inventory field: Inventory
    ///   3. has_item method on Inventory
    pub fn resolve_field_chain_type(
        &self,
        object: &Expression,
        field: &str
    ) -> Option<Type> {
        let object_type = self.infer_expression_type(object)?;
        let struct_name = self.get_base_type_name(&object_type)?;
        let field_types = self.struct_field_types.get(struct_name)?;
        field_types.get(field).cloned()
    }
    
    /// Look up a method signature on a given receiver type
    pub fn resolve_method_signature(
        &self,
        receiver_type: &Type,
        method: &str
    ) -> Option<&MethodSignature> {
        let type_name = self.get_base_type_name(receiver_type)?;
        
        // Check user-defined methods
        if let Some(methods) = self.method_signatures_by_type.get(type_name) {
            if let Some(sig) = methods.get(method) {
                return Some(sig);
            }
        }
        
        // Check external/stdlib methods
        if let Some(methods) = self.external_method_signatures.get(type_name) {
            if let Some(sig) = methods.get(method) {
                return Some(sig);
            }
        }
        
        None
    }
}
```

#### Step 3: Signature-Based Parameter Matching

```rust
impl MethodCallAnalyzer {
    pub fn should_add_ref(
        method: &str,
        arg: &Expression,
        param_idx: usize,
        // ... other params ...
        receiver_type: Option<&Type>, // NEW: Actual receiver type
    ) -> bool {
        // TRY 1: Look up actual method signature from receiver type
        if let Some(recv_type) = receiver_type {
            if let Some(sig) = type_analyzer.resolve_method_signature(recv_type, method) {
                let param_type = sig.param_types.get(param_idx)?;
                let arg_type = type_analyzer.infer_expression_type(arg)?;
                
                // PROPER DECISION BASED ON ACTUAL TYPES
                return match (param_type, arg_type) {
                    // Param wants &str, arg is String → add &
                    (Type::Reference(inner), Type::Custom(s)) 
                        if matches!(&**inner, Type::Custom(s) if s == "str") 
                        && s == "String" => true,
                    
                    // Param wants &T, arg is T (non-Copy) → add &
                    (Type::Reference(inner), arg_ty) 
                        if **inner == arg_ty && !is_copy_type(&arg_ty) => true,
                    
                    // Otherwise, don't add &
                    _ => false,
                };
            }
        }
        
        // TRY 2: Check global signature registry
        if let Some(sig) = global_signatures.get(method) {
            // Same logic as above
        }
        
        // FALLBACK: Conservative default (don't add &)
        // Log a warning that we couldn't resolve the signature
        eprintln!("Warning: No signature found for {}::{}", receiver_type_name, method);
        false
    }
}
```

### Migration Path

1. ✅ **Phase 1**: Add type registry enhancements (NEW fields)
2. ✅ **Phase 2**: Implement field-chain type resolution
3. ✅ **Phase 3**: Implement receiver-type-based signature lookup
4. ✅ **Phase 4**: Replace all heuristics with signature-based logic
5. ✅ **Phase 5**: Remove all hard-coded string matching
6. ✅ **Phase 6**: Add stdlib signature presets (Vec, HashMap, String, etc.)
7. ✅ **Phase 7**: Test with dialog.wj (should work without any game-specific code!)

### Stdlib Method Signatures (Preloaded)

Instead of heuristics, preload actual stdlib signatures:

```rust
fn init_stdlib_signatures() -> HashMap<String, HashMap<String, MethodSignature>> {
    let mut map = HashMap::new();
    
    // Vec<T> methods
    let mut vec_methods = HashMap::new();
    vec_methods.insert("push".to_string(), MethodSignature {
        param_types: vec![Type::Generic("T")], // Owned T
        param_ownership: vec![OwnershipMode::Owned],
        ...
    });
    vec_methods.insert("contains".to_string(), MethodSignature {
        param_types: vec![Type::Reference(Box::new(Type::Generic("T")))], // &T
        param_ownership: vec![OwnershipMode::Borrowed],
        ...
    });
    map.insert("Vec".to_string(), vec_methods);
    
    // String methods
    let mut string_methods = HashMap::new();
    string_methods.insert("contains".to_string(), MethodSignature {
        param_types: vec![Type::Reference(Box::new(Type::Custom("str")))], // &str
        param_ownership: vec![OwnershipMode::Borrowed],
        ...
    });
    map.insert("String".to_string(), string_methods);
    
    // HashMap<K, V> methods
    let mut map_methods = HashMap::new();
    map_methods.insert("get".to_string(), MethodSignature {
        param_types: vec![Type::Reference(Box::new(Type::Generic("K")))], // &K
        param_ownership: vec![OwnershipMode::Borrowed],
        ...
    });
    map_methods.insert("insert".to_string(), MethodSignature {
        param_types: vec![Type::Generic("K"), Type::Generic("V")], // K, V (both owned)
        param_ownership: vec![OwnershipMode::Owned, OwnershipMode::Owned],
        ...
    });
    map.insert("HashMap".to_string(), map_methods);
    
    map
}
```

## Benefits of Proper Solution

1. ✅ **General-purpose**: Works for ANY method, ANY naming convention
2. ✅ **Type-safe**: Based on actual types, not guesses
3. ✅ **Maintainable**: No need to add method names for every project
4. ✅ **Correct**: Uses actual signatures, not heuristics
5. ✅ **Windjammer philosophy**: Compiler understands structure
6. ✅ **Extensible**: Easy to add stdlib/crate signatures
7. ✅ **No game code in compiler**: Completely general-purpose

## Timeline

- **Short-term** (this session): Document all heuristics, create TDD tests
- **Medium-term** (next session): Implement type registry enhancements
- **Long-term** (future): Complete stdlib signature library

## Comparison

### Before (Heuristics)
```rust
// WRONG: Guessing based on method name
if matches!(method, "push" | "insert" | "append") {
    return false; // Don't add &
}
if method_name.starts_with("has_") && arg_name.ends_with("_id") {
    return true; // Add &
}
```

### After (Type-Based)
```rust
// CORRECT: Based on actual types
let sig = resolve_method_signature(receiver_type, method)?;
let param_type = sig.param_types[param_idx];
let arg_type = infer_expression_type(arg)?;
match (param_type, arg_type) {
    (Type::Reference(inner), Type::Custom(s)) if inner == "str" && s == "String" => true,
    _ => false,
}
```

## Testing Strategy

1. Create TDD tests for type resolution (field chains, method lookup)
2. Test with stdlib methods (Vec::push, String::contains, HashMap::get)
3. Test with user-defined types (GameState, Inventory, Player)
4. Verify NO game-specific code needed in compiler
5. Ensure all 47+ heuristics are replaced

## Success Criteria

- [ ] Zero hard-coded method name strings in method_call_analyzer.rs
- [ ] Zero hard-coded argument name strings
- [ ] Zero hard-coded field name strings  
- [ ] Zero game/UI-specific logic
- [ ] All decisions based on actual type signatures
- [ ] dialog.wj compiles without any heuristics
- [ ] Stdlib methods work via preloaded signatures
