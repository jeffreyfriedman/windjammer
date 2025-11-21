# Automatic Reference Insertion Design

## Problem

**Current Behavior**:
```windjammer
fn double(x: int) -> int {  // Analyzer infers: x: &i64
    x * 2
}

fn main() {
    let result = double(5)  // Generates: double(5)
}
```

**Generated Rust**:
```rust
fn double(x: &i64) -> i64 {  // ✅ Correct
    x * 2
}

fn main() {
    let result = double(5);  // ❌ Error: expected &i64, found integer
}
```

**Root Cause**: Code generator doesn't know that `double` expects `&i64`, so it doesn't insert `&` at call site.

---

## Solution Architecture

### Three-Component System

```
┌─────────────┐
│  Analyzer   │ → Infers ownership for all functions
└──────┬──────┘
       │ FunctionSignatures { name → [param ownership] }
       ↓
┌─────────────┐
│  Registry   │ → Stores inferred signatures
└──────┬──────┘
       │ Lookup during codegen
       ↓
┌─────────────┐
│  CodeGen    │ → Inserts & or &mut at call sites
└─────────────┘
```

### Component 1: Enhanced Analyzer Output

**Current**:
```rust
pub struct AnalyzedFunction {
    pub decl: FunctionDecl,
    pub inferred_ownership: HashMap<String, OwnershipMode>,  // param name → mode
}
```

**Needed**:
```rust
pub struct FunctionSignature {
    pub name: String,
    pub param_ownership: Vec<OwnershipMode>,  // Positional ownership modes
    pub return_ownership: OwnershipMode,
}

pub struct AnalysisResult {
    pub functions: Vec<AnalyzedFunction>,
    pub signatures: HashMap<String, FunctionSignature>,  // NEW: name → signature
}
```

### Component 2: Signature Registry

**Purpose**: Global lookup table for function signatures

```rust
pub struct SignatureRegistry {
    signatures: HashMap<String, FunctionSignature>,
}

impl SignatureRegistry {
    pub fn new() -> Self {
        // Pre-populate with stdlib functions
        let mut registry = SignatureRegistry {
            signatures: HashMap::new(),
        };
        
        // Add known functions
        registry.add_builtin("println", vec![OwnershipMode::Borrowed]);
        registry.add_builtin("format", vec![OwnershipMode::Borrowed]);
        
        registry
    }
    
    pub fn add_function(&mut self, name: String, sig: FunctionSignature) {
        self.signatures.insert(name, sig);
    }
    
    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(name)
    }
}
```

### Component 3: Smart Call Site Generation

**Current**:
```rust
Expression::Call { function, arguments } => {
    let func_str = self.generate_expression(function);
    let args: Vec<String> = arguments.iter()
        .map(|(_label, arg)| self.generate_expression(arg))
        .collect();
    format!("{}({})", func_str, args.join(", "))
}
```

**Enhanced**:
```rust
Expression::Call { function, arguments } => {
    let func_name = self.extract_function_name(function);
    let func_str = self.generate_expression(function);
    
    // Look up signature
    let signature = self.signature_registry.get_signature(&func_name);
    
    let args: Vec<String> = arguments.iter().enumerate()
        .map(|(i, (_label, arg))| {
            let arg_str = self.generate_expression(arg);
            
            // Check if this parameter expects a borrow
            if let Some(sig) = signature {
                if let Some(&ownership) = sig.param_ownership.get(i) {
                    match ownership {
                        OwnershipMode::Borrowed => {
                            // Check if arg is already a reference
                            if !self.is_reference_expression(arg) {
                                return format!("&{}", arg_str);
                            }
                        }
                        OwnershipMode::MutBorrowed => {
                            if !self.is_reference_expression(arg) {
                                return format!("&mut {}", arg_str);
                            }
                        }
                        OwnershipMode::Owned => {
                            // No change needed
                        }
                    }
                }
            }
            
            arg_str
        })
        .collect();
    
    format!("{}({})", func_str, args.join(", "))
}
```

---

## Implementation Steps

### Step 1: Modify Analyzer to Build Signature Registry

```rust
// src/analyzer.rs
impl Analyzer {
    pub fn analyze_program(
        &mut self,
        program: &Program
    ) -> (Vec<AnalyzedFunction>, SignatureRegistry) {
        let mut analyzed = Vec::new();
        let mut registry = SignatureRegistry::new();
        
        // Analyze each function
        for item in &program.items {
            if let Item::Function(func) = item {
                let analyzed_func = self.analyze_function(func);
                
                // Build signature from analysis
                let signature = self.build_signature(&analyzed_func);
                registry.add_function(func.name.clone(), signature);
                
                analyzed.push(analyzed_func);
            }
        }
        
        (analyzed, registry)
    }
    
    fn build_signature(&self, func: &AnalyzedFunction) -> FunctionSignature {
        let param_ownership: Vec<OwnershipMode> = func.decl.parameters
            .iter()
            .map(|param| {
                func.inferred_ownership
                    .get(&param.name)
                    .cloned()
                    .unwrap_or(OwnershipMode::Owned)
            })
            .collect();
        
        FunctionSignature {
            name: func.decl.name.clone(),
            param_ownership,
            return_ownership: OwnershipMode::Owned, // For now
        }
    }
}
```

### Step 2: Pass Registry to CodeGen

```rust
// src/main.rs
fn build_project(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // ... parse ...
    
    // Analyze with signature extraction
    let (analyzed_functions, signature_registry) = analyzer.analyze_program(&program);
    
    // Generate code with registry
    let mut codegen = CodeGenerator::new(signature_registry);
    let rust_code = codegen.generate_program(&program, &analyzed_functions);
    
    // ... write output ...
}
```

### Step 3: Update CodeGenerator

```rust
// src/codegen.rs
pub struct CodeGenerator {
    indent_level: usize,
    signature_registry: SignatureRegistry,  // NEW
}

impl CodeGenerator {
    pub fn new(registry: SignatureRegistry) -> Self {
        CodeGenerator {
            indent_level: 0,
            signature_registry: registry,
        }
    }
    
    fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Call { function, arguments } => {
                self.generate_call(function, arguments)
            }
            // ... other cases ...
        }
    }
    
    fn generate_call(
        &mut self,
        function: &Expression,
        arguments: &[(Option<String>, Expression)]
    ) -> String {
        // Extract function name
        let func_name = match function {
            Expression::Identifier(name) => name.clone(),
            Expression::FieldAccess { field, .. } => field.clone(),
            _ => String::new(), // Method calls, etc.
        };
        
        let func_str = self.generate_expression(function);
        
        // Look up signature
        let signature = self.signature_registry.get_signature(&func_name);
        
        // Generate arguments with automatic referencing
        let args: Vec<String> = arguments.iter().enumerate()
            .map(|(i, (_label, arg))| {
                self.generate_argument(arg, signature, i)
            })
            .collect();
        
        format!("{}({})", func_str, args.join(", "))
    }
    
    fn generate_argument(
        &mut self,
        arg: &Expression,
        signature: Option<&FunctionSignature>,
        position: usize
    ) -> String {
        let arg_str = self.generate_expression(arg);
        
        // Check if we need to insert a reference
        if let Some(sig) = signature {
            if let Some(&ownership) = sig.param_ownership.get(position) {
                match ownership {
                    OwnershipMode::Borrowed => {
                        // Insert & if not already a reference
                        if !self.is_reference_expr(arg) {
                            return format!("&{}", arg_str);
                        }
                    }
                    OwnershipMode::MutBorrowed => {
                        if !self.is_reference_expr(arg) {
                            return format!("&mut {}", arg_str);
                        }
                    }
                    OwnershipMode::Owned => {
                        // May need to clone or move
                    }
                }
            }
        }
        
        arg_str
    }
    
    fn is_reference_expr(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Unary { op: UnaryOp::Ref, .. }
        )
    }
}
```

---

## Edge Cases

### Case 1: Already Borrowed Argument
```windjammer
let x = 5
let result = double(&x)  // User explicitly passes &
```

**Solution**: Check if expression is already `UnaryOp::Ref`, don't add another `&`

### Case 2: Method Calls
```windjammer
let text = "hello"
text.len()  // self is already borrowed
```

**Solution**: Method receivers are handled separately, don't auto-reference

### Case 3: Pipe Operator
```windjammer
5 |> double  // Should become double(&5)
```

**Solution**: Pipe operator creates Call expression, goes through same logic

### Case 4: Variables vs Literals
```windjammer
double(5)      // Needs &5
double(x)      // Needs &x
double(x + 1)  // Needs &(x + 1)... complex!
```

**Solution**: For complex expressions, may need temp variable:
```rust
let _tmp = x + 1;
double(&_tmp)
```

### Case 5: Unknown Functions (External Crates)
```windjammer
external_func(x)  // Don't know signature
```

**Solution**: 
- Default to no auto-referencing for unknown functions
- User must be explicit: `external_func(&x)`
- Or: use type inference from Rust compilation errors (future)

---

## Testing Strategy

### Test 1: Basic Auto-Reference
```windjammer
fn double(x: int) -> int { x * 2 }
fn main() {
    let result = double(5)  // Should generate: double(&5)
}
```

### Test 2: Mutable Borrow
```windjammer
fn increment(x: int) { x += 1 }  // Infers &mut
fn main() {
    let mut x = 5
    increment(x)  // Should generate: increment(&mut x)
}
```

### Test 3: Ownership Transfer
```windjammer
fn consume(x: int) -> int { x }  // Infers owned (returned)
fn main() {
    let x = 5
    let y = consume(x)  // Should generate: consume(x) - no &
}
```

### Test 4: Pipe Operator
```windjammer
fn double(x: int) -> int { x * 2 }
fn main() {
    let result = 5 |> double  // Should generate: double(&5)
}
```

### Test 5: Complex Expressions
```windjammer
fn process(x: int) { println!("{}", x) }
fn main() {
    process(5 + 10)  // Should generate: process(&(5 + 10)) or temp var
}
```

---

## Performance Considerations

**Overhead**:
- HashMap lookups for every function call during codegen
- ~O(1) per call, negligible impact

**Memory**:
- SignatureRegistry size: ~few KB for typical programs
- Acceptable overhead

**Optimization**:
- Cache signature lookups
- Use interned strings for function names

---

## Migration Strategy

### Phase 1: Implement Core (Now)
- Basic auto-reference for simple cases
- Test with hello_world example

### Phase 2: Handle Edge Cases (v0.2)
- Complex expressions
- Method calls
- External functions

### Phase 3: Optimization (v0.3)
- Performance tuning
- Better heuristics
- User control (opt-out)

---

## User Control (Future)

Allow users to opt-out of auto-referencing:
```windjammer
@no_auto_ref
fn my_function() {
    // Manual control over references
}
```

Or mark specific functions:
```windjammer
@explicit_refs
fn external_api_call(x: &int) {
    // Must pass &x explicitly
}
```

---

## Success Criteria

**Must Work**:
- ✅ Basic function calls with inferred borrows
- ✅ Pipe operator with auto-reference
- ✅ Both & and &mut insertion

**Should Work**:
- ✅ Method calls (self parameters)
- ✅ Complex expressions
- ✅ Nested function calls

**Nice to Have**:
- ✅ Optimization for common patterns
- ✅ User override controls
- ✅ IDE hints showing inserted references

---

*Status: Design Complete*  
*Priority: P0 (Blocker)*  
*Estimated Effort: 2-3 days*  
*Next Step: Implement Step 1 (Analyzer changes)*

