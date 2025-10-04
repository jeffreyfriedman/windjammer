# Generics Implementation Plan

## Goal
Add basic generics support to Windjammer to enable:
1. Generic functions: `fn identity<T>(x: T) -> T`
2. Generic structs: `struct Box<T> { value: T }`
3. Turbofish syntax: `Vec::<i32>::new()`, `Option::<String>::None`
4. Type parameters in function calls and expressions

## Current Blockers
Stdlib modules fail because of:
- ❌ `DateTime::<Utc>::from_timestamp()` - turbofish syntax
- ❌ `Vec<T>`, `Option<T>`, `Result<T, E>` - generic types
- ❌ Generic function parameters and return types

## Implementation Steps

### 1. AST Extensions

Add type parameters to structs and functions:

```rust
pub struct FunctionDecl {
    pub name: String,
    pub type_params: Vec<String>,  // NEW: ["T", "U"]
    pub parameters: Vec<(String, Type, OwnershipHint)>,
    pub return_type: Option<Type>,
    // ... existing fields
}

pub struct StructDecl {
    pub name: String,
    pub type_params: Vec<String>,  // NEW: ["T"]
    pub fields: Vec<StructField>,
    // ... existing fields
}

pub struct ImplBlock {
    pub type_params: Vec<String>,  // NEW: ["T"]
    pub type_name: String,
    pub trait_name: Option<String>,
    pub functions: Vec<FunctionDecl>,
    // ... existing fields
}

// Extend Type enum
pub enum Type {
    // ... existing variants
    Generic(String),  // NEW: T, U, etc.
    Parameterized(Box<Type>, Vec<Type>),  // NEW: Vec<T>, Option<String>
}
```

### 2. Lexer
Already supports `<` and `>` tokens ✅

### 3. Parser Updates

#### A. Parse Type Parameters
```rust
fn parse_type_params(&mut self) -> Result<Vec<String>, String> {
    // <T> or <T, U> or <T, U, V>
    if self.current_token() != &Token::Lt {
        return Ok(Vec::new());
    }
    
    self.advance(); // consume <
    let mut params = Vec::new();
    
    loop {
        if let Token::Ident(name) = self.current_token() {
            params.push(name.clone());
            self.advance();
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else if self.current_token() == &Token::Gt {
                self.advance();
                break;
            } else {
                return Err("Expected ',' or '>' in type parameters".to_string());
            }
        } else {
            return Err("Expected type parameter name".to_string());
        }
    }
    
    Ok(params)
}
```

#### B. Update Function Parsing
```rust
fn parse_function(&mut self) -> Result<FunctionDecl, String> {
    let name = ...;
    
    // NEW: Parse type parameters: fn foo<T>(...)
    let type_params = self.parse_type_params()?;
    
    // ... rest of function parsing
    
    Ok(FunctionDecl {
        name,
        type_params,  // NEW
        // ... other fields
    })
}
```

#### C. Update Struct Parsing
```rust
fn parse_struct(&mut self) -> Result<StructDecl, String> {
    let name = ...;
    
    // NEW: Parse type parameters: struct Box<T> {
    let type_params = self.parse_type_params()?;
    
    // ... rest of struct parsing
    
    Ok(StructDecl {
        name,
        type_params,  // NEW
        // ... other fields
    })
}
```

#### D. Parse Turbofish in Expressions
```rust
// In parse_primary_expression, after parsing identifier:
// Check for turbofish: Type::<T>::method()

if self.current_token() == &Token::DoubleColon {
    self.advance();
    
    // Check for turbofish
    if self.current_token() == &Token::Lt {
        let type_args = self.parse_type_arguments()?;
        // Store turbofish info in expression
    }
}
```

#### E. Parse Parameterized Types
```rust
fn parse_type(&mut self) -> Result<Type, String> {
    let base_type = match self.current_token() {
        Token::Ident(name) => Type::Custom(name.clone()),
        // ... other cases
    };
    
    self.advance();
    
    // NEW: Check for type parameters: Vec<T>
    if self.current_token() == &Token::Lt {
        self.advance();
        let mut type_args = Vec::new();
        
        loop {
            type_args.push(self.parse_type()?);
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else if self.current_token() == &Token::Gt {
                self.advance();
                break;
            } else {
                return Err("Expected ',' or '>' in type arguments".to_string());
            }
        }
        
        return Ok(Type::Parameterized(Box::new(base_type), type_args));
    }
    
    Ok(base_type)
}
```

### 4. Codegen Updates

Generics are **transparent** - just pass them through to Rust:

```rust
fn generate_function(&mut self, func: &FunctionDecl) -> String {
    let mut output = String::new();
    
    output.push_str("fn ");
    output.push_str(&func.name);
    
    // NEW: Add type parameters
    if !func.type_params.is_empty() {
        output.push('<');
        output.push_str(&func.type_params.join(", "));
        output.push('>');
    }
    
    // ... rest of codegen
}

fn generate_struct(&mut self, s: &StructDecl) -> String {
    let mut output = String::new();
    
    output.push_str("struct ");
    output.push_str(&s.name);
    
    // NEW: Add type parameters
    if !s.type_params.is_empty() {
        output.push('<');
        output.push_str(&s.type_params.join(", "));
        output.push('>');
    }
    
    // ... rest of codegen
}

fn type_to_rust(&self, t: &Type) -> String {
    match t {
        Type::Generic(name) => name.clone(),  // NEW: T -> T
        Type::Parameterized(base, args) => {  // NEW: Vec<T> -> Vec<T>
            format!(
                "{}<{}>",
                self.type_to_rust(base),
                args.iter().map(|t| self.type_to_rust(t)).collect::<Vec<_>>().join(", ")
            )
        }
        // ... other cases
    }
}
```

### 5. Testing

Create test files:

```windjammer
// tests/fixtures/generics_basic.wj
fn identity<T>(x: T) -> T {
    x
}

fn swap<T, U>(a: T, b: U) -> (U, T) {
    (b, a)
}

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

fn main() {
    let x = identity(5)
    let b = Box::new("hello")
    let val = b.get()
}
```

```windjammer
// tests/fixtures/turbofish.wj
fn main() {
    let v = Vec::<i32>::new()
    let opt = Option::<String>::None
    let result = Result::<i32, String>::Ok(42)
}
```

## Success Criteria

✅ Generic functions compile
✅ Generic structs compile
✅ Turbofish syntax parses and generates correct Rust
✅ Parameterized types work: `Vec<T>`, `Option<T>`, `Result<T, E>`
✅ Generic impl blocks work
✅ stdlib modules can use chrono's `DateTime<Utc>`
✅ Type parameters pass through to Rust unchanged

## Timeline

- **Phase 1** (2 hours): AST extensions + basic parsing
- **Phase 2** (2 hours): Turbofish syntax + parameterized types
- **Phase 3** (1 hour): Codegen updates
- **Phase 4** (1 hour): Testing + stdlib fixes

**Total**: ~6 hours of focused work

## Next Steps

1. Start with AST extensions
2. Add type parameter parsing
3. Update function/struct/impl parsing
4. Add turbofish to expression parsing
5. Update codegen to pass through generics
6. Test with stdlib modules
7. Fix std/time, std/json, std/math, etc.
