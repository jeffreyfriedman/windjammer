# Type and Trait Inference Design for Windjammer

## Executive Summary

This document outlines Windjammer's strategy for achieving "80% simplicity through 80% inference" - allowing most developers to write code without explicit type annotations, trait bounds, or lifetime specifications, while maintaining full Rust safety guarantees.

**Core Philosophy**: Progressive disclosure of complexity through compiler intelligence, not feature limitation.

---

## Research: Inference in Other Languages

### Hindley-Milner Type Inference (ML, Haskell, OCaml)

**Algorithm**:
- Bottom-up type inference from expressions
- Unification of type variables with constraints
- Generalization at let-bindings

**Example (Haskell)**:
```haskell
-- No type annotation needed
map f [] = []
map f (x:xs) = f x : map f xs

-- Inferred type: (a -> b) -> [a] -> [b]
```

**Strengths**:
- Complete type inference for parametric polymorphism
- No annotations needed in most cases
- Provably sound and complete

**Limitations**:
- Doesn't handle subtyping well
- Requires explicit type classes for ad-hoc polymorphism
- Can't infer higher-rank types without annotations

**Relevance to Windjammer**:
✅ Can adapt for trait bound inference
✅ Use constraint collection and unification
❌ Rust's trait system is more complex than Haskell's type classes

### Local Type Inference (Java, C#, Scala)

**Algorithm**:
- Type flows from expressions to declarations
- Limited scope (within methods)
- Explicit annotations required at boundaries

**Example (Java 10+)**:
```java
var list = new ArrayList<String>();  // Type inferred as ArrayList<String>
var x = list.get(0);                 // Type inferred as String
```

**Strengths**:
- Simple and predictable
- Fast compilation
- Works with existing OOP type systems

**Limitations**:
- Only local inference (within method)
- Method signatures must be explicit
- No inference across function boundaries

**Relevance to Windjammer**:
✅ Good for local variable types (we already do this via Rust)
⚠️ Too limited for our goals (we want function-level inference)

### Protocol/Trait Inference (Swift, Kotlin)

**Swift Protocol Requirements**:
```swift
func printAll<T>(_ items: [T]) {
    for item in items {
        print(item)  // Error: T doesn't conform to CustomStringConvertible
    }
}

// Must be explicit:
func printAll<T: CustomStringConvertible>(_ items: [T]) {
    for item in items {
        print(item)  // OK
    }
}
```

**Swift's Approach**:
- Explicit protocol conformance required
- No automatic inference of protocol bounds
- Compile-time error if constraint not met

**Kotlin's Approach**:
```kotlin
inline fun <reified T> printAll(items: List<T>) {
    items.forEach { println(it) }  // Works due to reified generics
}
```

**Strengths (Swift)**:
- Clear and explicit contracts
- Predictable behavior
- Good error messages

**Limitations (Swift)**:
- Verbose for simple cases
- Requires annotations even when "obvious"

**Relevance to Windjammer**:
⚠️ This is what we want to improve upon!
✅ Shows that static languages typically require explicit bounds
✅ Windjammer can do better by analyzing usage

### Gradual Typing (TypeScript, Python type hints)

**TypeScript's Approach**:
```typescript
// Can omit types, inferred as 'any' or from context
function map(arr, fn) {  // Inferred types from usage
    return arr.map(fn);
}

// Or be explicit
function map<T, U>(arr: T[], fn: (x: T) => U): U[] {
    return arr.map(fn);
}
```

**Strengths**:
- Flexible: can add types incrementally
- Practical for migration from untyped code
- Good IDE support through inference

**Limitations**:
- 'any' type escapes safety guarantees
- Not suitable for systems programming
- Runtime type errors possible

**Relevance to Windjammer**:
❌ Can't use gradual typing (need full safety)
✅ Idea of "optional annotations" is good
✅ Show inferred types in IDE

### Rust's Current Inference

**What Rust Infers**:
```rust
let x = 5;              // ✅ Type inferred as i32
let v = vec![1, 2, 3];  // ✅ Type inferred as Vec<i32>
let f = |x| x + 1;      // ✅ Closure type inferred from usage

// But NOT:
fn print_all<T>(items: Vec<T>) {
    for item in items {
        println!("{}", item);  // ❌ Error: T doesn't implement Display
    }
}
```

**Rust's Limitations**:
- No inference of trait bounds on generic parameters
- Function signatures must be explicit
- Lifetime inference only within functions

**Why Rust Doesn't Infer Trait Bounds**:
1. **Separate compilation**: Don't want to re-analyze callers when implementation changes
2. **Clear contracts**: Function signature is documentation
3. **Error locality**: Errors at function boundary, not call site
4. **Coherence**: Trait impl selection must be unambiguous

**Relevance to Windjammer**:
✅ These are real engineering trade-offs!
✅ We must address them in our design
✅ Our transpiler model gives us more flexibility

---

## Windjammer's Approach: Whole-Program Analysis

### Key Insight

**Windjammer is a transpiler, not a standalone compiler.**

This gives us unique advantages:
1. **Whole-program view**: Can analyze entire .wj file before generating Rust
2. **No separate compilation pressure**: Generate complete Rust signatures
3. **Error mapping**: Can translate Rust errors back to Windjammer
4. **Incremental adoption**: Complex cases can use explicit annotations

### Inference Algorithm Design

#### Phase 1: Constraint Collection

**Input**: Parsed AST with potentially missing trait bounds

**Process**: Walk the function body and collect trait requirements

```rust
// Example AST analysis
fn analyze_function(func: &FunctionDecl) -> Vec<TraitConstraint> {
    let mut constraints = Vec::new();
    
    for stmt in &func.body {
        match stmt {
            // println!("{}", x) requires Display
            Expression::Call(name: "println!", args) => {
                if format_string_has_display(args[0]) {
                    for arg in args[1..] {
                        constraints.push(TraitConstraint {
                            type_param: infer_type(arg),
                            trait_name: "Display"
                        });
                    }
                }
            },
            
            // println!("{:?}", x) requires Debug
            Expression::Call(name: "println!", args) => {
                if format_string_has_debug(args[0]) {
                    for arg in args[1..] {
                        constraints.push(TraitConstraint {
                            type_param: infer_type(arg),
                            trait_name: "Debug"
                        });
                    }
                }
            },
            
            // x + y requires Add
            Expression::Binary(op: BinaryOp::Plus, left, right) => {
                constraints.push(TraitConstraint {
                    type_param: infer_type(left),
                    trait_name: "Add<...>"
                });
            },
            
            // x == y requires PartialEq
            Expression::Binary(op: BinaryOp::Equals, left, right) => {
                constraints.push(TraitConstraint {
                    type_param: infer_type(left),
                    trait_name: "PartialEq"
                });
            },
            
            // x.clone() requires Clone
            Expression::MethodCall(obj, method: "clone") => {
                constraints.push(TraitConstraint {
                    type_param: infer_type(obj),
                    trait_name: "Clone"
                });
            },
            
            // for item in collection requires IntoIterator
            Statement::For { iterable, ... } => {
                constraints.push(TraitConstraint {
                    type_param: infer_type(iterable),
                    trait_name: "IntoIterator"
                });
            },
        }
    }
    
    constraints
}
```

#### Phase 2: Constraint Simplification

**Deduplicate and merge constraints**:
```rust
fn simplify_constraints(constraints: Vec<TraitConstraint>) -> Vec<TypeBound> {
    let mut bounds: HashMap<String, HashSet<String>> = HashMap::new();
    
    for constraint in constraints {
        bounds.entry(constraint.type_param)
              .or_insert_with(HashSet::new)
              .insert(constraint.trait_name);
    }
    
    // Convert to sorted bounds for stable output
    bounds.into_iter()
          .map(|(type_param, traits)| {
              let mut sorted_traits: Vec<_> = traits.into_iter().collect();
              sorted_traits.sort();
              TypeBound { type_param, traits: sorted_traits }
          })
          .collect()
}
```

#### Phase 3: Code Generation

**Generate Rust with inferred bounds**:
```rust
fn generate_function_with_bounds(func: &FunctionDecl, bounds: &[TypeBound]) -> String {
    let mut output = String::from("fn ");
    output.push_str(&func.name);
    
    // Generate type parameters with inferred bounds
    if !func.type_params.is_empty() || !bounds.is_empty() {
        output.push('<');
        
        for (i, type_param) in func.type_params.iter().enumerate() {
            if i > 0 { output.push_str(", "); }
            output.push_str(&type_param.name);
            
            // Find inferred bounds for this type parameter
            if let Some(bound) = bounds.iter().find(|b| b.type_param == type_param.name) {
                output.push_str(": ");
                output.push_str(&bound.traits.join(" + "));
            }
        }
        
        output.push('>');
    }
    
    // ... rest of function generation
}
```

### Trait Detection Rules

**Comprehensive mapping of usage → trait**:

| Usage Pattern | Required Trait | Detection Method |
|---------------|---------------|------------------|
| `println!("{}", x)` | `Display` | Format string analysis |
| `println!("{:?}", x)` | `Debug` | Format string analysis |
| `x + y` | `Add` | Binary operator |
| `x - y` | `Sub` | Binary operator |
| `x * y` | `Mul` | Binary operator |
| `x / y` | `Div` | Binary operator |
| `x == y` | `PartialEq` | Comparison operator |
| `x < y` | `PartialOrd` | Comparison operator |
| `x.clone()` | `Clone` | Method call |
| `x.to_string()` | `ToString` or `Display` | Method call |
| `for item in x` | `IntoIterator` | For loop |
| `x.iter()` | N/A (inherent method) | Method call |
| `Some(x)` | N/A (enum constructor) | Pattern match |
| Assignment `let y = x` | `Copy` (if by value) | Move analysis |
| Send to channel | `Send` | Channel operation |
| Share across threads | `Sync` | Thread spawn analysis |

### Edge Cases and Limitations

#### 1. **Ambiguous Constraints**

**Problem**:
```windjammer
fn process<T>(x: T) {
    // No usage of x!
}
```

**Solution**:
- If no constraints detected, generate no bounds
- Rust will error if called incorrectly
- Error mapping explains: "Add trait bounds to T"

#### 2. **Conflicting Constraints**

**Problem**:
```windjammer
fn impossible<T>(x: T) {
    println!("{}", x)      // Requires Display
    some_copy_only_fn(x)   // Requires Copy (Display types aren't usually Copy)
}
```

**Solution**:
- Infer both: `T: Display + Copy`
- Let Rust type checker reject if impossible
- Error mapping suggests: "T cannot satisfy both Display and Copy"

#### 3. **Method Ambiguity**

**Problem**:
```windjammer
fn call_method<T>(x: T) {
    x.custom_method()  // Which trait provides this?
}
```

**Solution**:
- Cannot infer trait for custom methods
- Require explicit bound: `T: MyTrait`
- Provide helpful error: "Cannot infer trait for custom_method(), specify T: TraitName"

#### 4. **Associated Types**

**Problem**:
```windjammer
fn iterate<T>(x: T) {
    for item in x {  // T: IntoIterator, but what's Item type?
        // ...
    }
}
```

**Solution**:
- Infer `T: IntoIterator`
- Let Rust's type inference handle `Item` type
- Works because Rust's inference operates on the generated code

#### 5. **Lifetime Inference**

**Problem**:
```windjammer
fn longest<T>(x: &T, y: &T) -> &T {
    if condition { x } else { y }
}
```

**Solution**:
- **Phase 1** (v0.10.0): Don't infer lifetimes, require explicit or rely on Rust elision
- **Phase 2** (v0.11.0+): Analyze return value dependencies, generate lifetime annotations
- **Phase 3**: Full lifetime inference (research project)

#### 6. **Higher-Rank Trait Bounds (HRTB)**

**Problem**:
```rust
// Rust: for<'a> F: Fn(&'a T) -> &'a U
```

**Solution**:
- Too complex for inference in v0.10.0
- Require explicit syntax (escape hatch)
- Future research: infer from closure usage patterns

---

## Implementation Strategy for v0.10.0

### Scope: Start Conservative

**Phase 1 - Ship in v0.10.0**:
- ✅ Infer trait bounds from standard library usage
  - `Display`, `Debug`, `Clone`, `Copy`, `PartialEq`, `PartialOrd`
  - Binary operators: `Add`, `Sub`, `Mul`, `Div`
  - Iteration: `IntoIterator`, `Iterator`
- ✅ Named bound sets (`bound Printable = Display + Debug`)
- ✅ Generate helpful errors when inference fails

**Phase 2 - v0.11.0**:
- Infer `Send` and `Sync` from concurrency usage
- Infer return types from function bodies
- Smart error propagation (infer `Result` return type from `?`)

**Phase 3 - Research for v0.12.0+**:
- Cross-function constraint inference
- Lifetime inference beyond Rust's elision rules
- Associated type inference

### Implementation Plan (v0.10.0)

**Week 1: Foundation**
1. Add `InferredBounds` phase to analyzer
2. Implement constraint collection for basic traits
3. Unit tests for each trait detection rule

**Week 2: Integration**
4. Integrate with codegen to generate bounds
5. Update error mapper to handle inference failures
6. Test with real examples

**Week 3: Named Bounds**
7. Parse `bound` declarations
8. Expand bound aliases in codegen
9. Documentation and examples

**Week 4: Polish**
10. Comprehensive testing
11. Error message improvements
12. Documentation: migration guide

### Testing Strategy

**Test Categories**:

1. **Positive Tests**: Inference works correctly
   ```windjammer
   fn print<T>(x: T) {
       println!("{}", x)
   }
   // Should generate: fn print<T: Display>(x: T)
   ```

2. **Negative Tests**: Inference doesn't over-constrain
   ```windjammer
   fn store<T>(x: T) -> T {
       x  // Should NOT add any bounds
   }
   // Should generate: fn store<T>(x: T) -> T
   ```

3. **Edge Cases**: Complex usage patterns
   ```windjammer
   fn complex<T>(x: T, y: T) -> T {
       if x == y { x.clone() } else { y }
   }
   // Should generate: fn complex<T: PartialEq + Clone>(x: T, y: T) -> T
   ```

4. **Error Cases**: Inference fails gracefully
   ```windjammer
   fn custom<T>(x: T) {
       x.custom_method()  // Error: Cannot infer trait
   }
   // Should error with helpful message
   ```

---

## Documentation Strategy

### User-Facing Documentation

**Beginner Tutorial**:
```windjammer
// Chapter 3: Working with Generics

// The simple way (Level 1) - no trait bounds!
fn print_all<T>(items: Vec<T>) {
    for item in items {
        println!("{}", item)  // Compiler figures out T needs Display
    }
}

// This just works:
print_all(vec![1, 2, 3])
print_all(vec!["a", "b", "c"])

// Behind the scenes, Windjammer infers:
// fn print_all<T: Display>(items: Vec<T>) { ... }
```

**Advanced Tutorial**:
```windjammer
// Chapter 10: Explicit Trait Bounds

// When you need more control (Level 2):
fn process<T: Display + Clone + Send>(x: T) {
    // Explicit bounds for:
    // - Documentation
    // - Complex constraints  
    // - When inference fails
}
```

### Developer Documentation

**In `docs/INFERENCE_DESIGN.md`** (this document):
- Algorithm details
- Edge cases and limitations
- Future research directions

**In `docs/CONTRIBUTING.md`**:
- How to add new trait detection rules
- How to test inference
- How to debug inference failures

---

## Future Research Directions

### 1. Cross-Function Inference

**Challenge**:
```windjammer
fn helper<T>(x: T) {
    println!("{}", x)  // Requires Display
}

fn caller<T>(x: T) {
    helper(x)  // Should propagate Display requirement to caller's T
}
```

**Approach**:
- Build call graph
- Propagate constraints across function boundaries
- Requires fixpoint iteration

**Status**: Research for v0.12.0+

### 2. Lifetime Inference

**Challenge**:
```windjammer
fn longest<T>(x: &T, y: &T) -> &T {
    if condition { x } else { y }
}
// Should infer lifetime: fn longest<'a, T>(x: &'a T, y: &'a T) -> &'a T
```

**Approach**:
- Analyze return value dependencies
- Build lifetime constraint graph
- Generate minimal lifetime annotations

**Status**: Research for v0.11.0+

### 3. Effect Inference

**Challenge**:
```windjammer
fn process<T>(x: T) {
    spawn_thread(move || { use(x); })  // Requires T: Send
}
```

**Approach**:
- Track effect contexts (thread spawn, async, etc.)
- Infer marker traits (`Send`, `Sync`, `'static`)
- Propagate effects through closures

**Status**: v0.11.0

---

## Success Metrics

**Quantitative**:
- 80% of generic functions have no explicit trait bounds in examples
- 90% of inference cases succeed without fallback to explicit bounds
- <5% performance overhead in compilation time from inference

**Qualitative**:
- Beginner tutorials don't mention trait bounds until chapter 10
- User feedback: "I don't think about traits, it just works"
- Error messages are helpful when inference fails

---

## Conclusion

**Windjammer's inference strategy is technically sound and achievable.**

Key advantages:
- ✅ Whole-program analysis (transpiler model)
- ✅ Can generate complete Rust signatures
- ✅ Progressive disclosure (implicit by default, explicit when needed)
- ✅ Escape hatch (use explicit bounds or raw Rust)

Key risks:
- ⚠️ Compilation time overhead (mitigated by simple analysis)
- ⚠️ Complex error messages when inference fails (mitigated by error mapping)
- ⚠️ User confusion about "magic" (mitigated by documentation)

**This is the path to true "80% simplicity, 80% power"** - not by limiting features, but by making them automatic through intelligence.

---

## References

**Academic**:
- Damas, Luis, and Robin Milner. "Principal type-schemes for functional programs." POPL 1982.
- Pierce, Benjamin C. "Types and programming languages." MIT press, 2002.
- Pierce, Benjamin C., and David N. Turner. "Local type inference." ACM TOPLAS, 2000.

**Industry**:
- Rust RFC 1214: Clarify (and improve) rules for projections and well-formedness
- Swift Evolution: Protocol-Oriented Programming
- TypeScript Handbook: Type Inference
- Kotlin Language Specification: Type Inference

**Prior Art**:
- Haskell's type class system
- OCaml's module system
- Scala's implicit resolution
- C++ Concepts (trait constraints)

---

**Status**: Design Complete, Ready for v0.10.0 Implementation  
**Authors**: Windjammer Core Team  
**Last Updated**: 2025-10-06

