# TODO: Support Module Declarations in Parser

**Priority**: HIGH (language completeness)  
**Estimated Time**: 1-2 hours  
**Status**: BLOCKING for proper module system

---

## 🚨 **THE PROBLEM**

Parser does not support module declarations:

```windjammer
pub mod math
pub mod rendering
pub mod physics
```

**Current Error**: `Parse error: Unexpected token: Mod (at token position 1)`

**Impact**:
- Can't write proper `mod.wj` files
- Can't organize code into module hierarchies
- Have to manually create `mod.rs` files in build scripts
- Not a complete language!

---

## ✅ **THE FIX**

### **Grammar to Support**

```windjammer
// Simple module declaration
mod math;

// Public module
pub mod rendering;

// Inline module (future - more complex)
mod utils {
    pub fn helper() {}
}
```

---

## 🔧 **IMPLEMENTATION**

### **Phase 1: Lexer (Already Done?)**

The lexer should already handle `Mod` token. Verify:

```rust
// In lexer.rs
Token::Mod => "mod",
```

### **Phase 2: Parser - Add Module Declaration**

**File**: `src/parser/item_parser.rs`

```rust
// In parse_item() method
Token::Mod => {
    self.advance(); // consume 'mod'
    
    let name = self.expect_identifier()?;
    
    // Check for inline module or declaration
    if self.current_token() == &Token::LBrace {
        // Inline module: mod math { ... }
        self.advance(); // consume '{'
        let items = self.parse_items_until(&Token::RBrace)?;
        self.expect(Token::RBrace)?;
        
        Ok(Item::Mod {
            name,
            is_pub: is_pub,
            items,
            location: Some(start_location),
        })
    } else {
        // Module declaration: mod math;
        self.expect(Token::Semicolon)?;
        
        Ok(Item::Mod {
            name,
            is_pub: is_pub,
            items: vec![], // External module file
            location: Some(start_location),
        })
    }
}
```

### **Phase 3: AST - Update Item Enum**

**File**: `src/parser/ast.rs`

Check if `Item::Mod` already exists:

```rust
pub enum Item {
    // ... other variants ...
    Mod {
        name: String,
        is_pub: bool,
        items: Vec<Item>, // Empty for external modules
        location: Option<Location>,
    },
    // ... other variants ...
}
```

### **Phase 4: Code Generator - Generate mod declarations**

**File**: `src/codegen/rust/generator.rs`

```rust
// In generate_item() or similar
Item::Mod { name, is_pub, items, .. } => {
    if items.is_empty() {
        // External module declaration
        if *is_pub {
            format!("pub mod {};\n", name)
        } else {
            format!("mod {};\n", name)
        }
    } else {
        // Inline module
        let mut output = String::new();
        if *is_pub {
            output.push_str("pub ");
        }
        output.push_str(&format!("mod {} {{\n", name));
        
        for item in items {
            output.push_str(&self.generate_item(item));
        }
        
        output.push_str("}\n");
        output
    }
}
```

---

## 🧪 **TESTING**

### **Test 1: Simple Module Declaration**

**File**: `tests/module_declaration_test.wj`

```windjammer
pub mod math;
pub mod physics;
mod internal;
```

**Expected Rust Output**:
```rust
pub mod math;
pub mod physics;
mod internal;
```

### **Test 2: Inline Module (Future)**

```windjammer
pub mod utils {
    pub fn helper() -> int {
        42
    }
}
```

**Expected Rust Output**:
```rust
pub mod utils {
    pub fn helper() -> i32 {
        42
    }
}
```

### **Test 3: Nested Modules (Future)**

```windjammer
pub mod game {
    pub mod physics {
        pub struct World {}
    }
}
```

---

## 📋 **IMPLEMENTATION CHECKLIST**

- [ ] Verify `Token::Mod` exists in lexer
- [ ] Add module parsing to `item_parser.rs`
- [ ] Update or verify `Item::Mod` in AST
- [ ] Add module code generation to Rust generator
- [ ] Add module code generation to JavaScript generator
- [ ] Write `tests/module_declaration_test.wj`
- [ ] Write `tests/inline_module_test.wj` (future)
- [ ] Run full test suite
- [ ] Test with `windjammer-game/windjammer-game-core/src_wj/mod.wj`

---

## 🎯 **SUCCESS CRITERIA**

1. ✅ `mod math;` parses without error
2. ✅ `pub mod rendering;` generates correct Rust
3. ✅ Inline modules parse and generate correctly
4. ✅ `mod.wj` files compile successfully
5. ✅ All existing tests still pass

---

## 💡 **WHY IT MATTERS**

### **Proper Module System**
- Windjammer needs a complete module system
- Can't rely on manual `mod.rs` generation forever
- Users expect `mod` keyword to work!

### **Language Completeness**
- This is basic Rust syntax
- Should be supported from day 1
- Dogfooding revealed this gap

### **Better Code Organization**
- Enables proper project structure
- Reduces build script complexity
- More idiomatic Windjammer code

---

## 📝 **CURRENT WORKAROUND**

**Status**: Temporary files deleted
- Deleted `windjammer-game-core/src_wj/mod.wj`
- Deleted `windjammer-game-core/src_wj/runtime.wj`
- Build script manually generates `mod.rs`

**Reason**: Allows platformer to build while we fix the parser properly

---

## 🔗 **RELATED**

- **Similar Issue**: `extern fn` declarations in inline `mod` blocks (also not supported)
- **Future Work**: Module path resolution (`use crate::module::Type`)
- **Dogfooding Win**: This bug was discovered through real-world use!

---

**Fix this after platformer works!** 🎮

Then dogfooding will validate the fix automatically when we restore `mod.wj`.









