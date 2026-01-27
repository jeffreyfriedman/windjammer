# Error Message Design Research & Guidelines

## Research: Best Practices from Leading Compilers

### 1. **Rust** - The Gold Standard

**Philosophy:** "Helpful, not pedantic"
- Multi-line error display with source context
- Color-coded severity (error/warning/note/help)
- Precise source spans with ^ pointers
- Numbered error codes (E0308, etc.) linking to documentation
- Contextual help suggestions
- "Did you mean...?" for typos

**Example:**
```
error[E0308]: mismatched types
  --> src/main.rs:5:18
   |
5  |     let x: i32 = "hello";
   |            ---   ^^^^^^^ expected `i32`, found `&str`
   |            |
   |            expected due to this
   |
help: try converting the string to an integer
   |
5  |     let x: i32 = "hello".parse()?;
   |                  ++++++++++++++++
```

**Key Insights:**
- Show the TYPE CONSTRAINT first (what was expected)
- Show the ACTUAL VALUE second (what was found)
- Explain WHY the error occurred
- Suggest HOW to fix it
- Link to documentation for learning

---

### 2. **Elm** - The Friendliest Compiler

**Philosophy:** "Assume the user is smart but made a simple mistake"
- Conversational tone ("I see...")
- Explains the problem in plain English
- Educational hints about common patterns
- ASCII art for clarity

**Example:**
```
-- TYPE MISMATCH ------------------------------------------------- src/Main.elm

The 1st argument to `map` is not what I expect:

4|   List.map String.length [1, 2, 3]
              ^^^^^^^^^^^^^
This `String.length` value is a:

    String -> Int

But `map` needs the 1st argument to be:

    number -> a

Hint: I always figure out the type of arguments from left to right. If an
argument is acceptable when I check it, I assume it is "correct" in subsequent
checks. So the problem may actually be how `map` is used, not how `String.length`
is defined.
```

**Key Insights:**
- Use natural language, not jargon
- Explain what YOU (the compiler) are thinking
- Anticipate common mistakes and address them
- Teach patterns, not just fix errors

---

### 3. **TypeScript** - Incremental Improvement

**Philosophy:** "Progressive enhancement"
- Started with basic errors, improved over time
- Quick fixes integrated into IDE
- Type narrowing explanations
- Union/intersection type feedback

**Example:**
```
error TS2322: Type 'string' is not assignable to type 'number'.

15 let x: number = "hello";
   ~

Did you mean to write `let x = "hello"`?
```

**Key Insights:**
- Start simple, improve incrementally
- Integration with tooling (LSP) is critical
- Quick fixes are more valuable than perfect diagnostics
- Show type flow for complex scenarios

---

### 4. **Clang/LLVM** - C++ Compiler

**Philosophy:** "Precise and actionable"
- Fix-it hints with exact code replacements
- Template error reduction (don't show 100 lines)
- Macro expansion traces
- Verbose mode for deep debugging

**Example:**
```
main.cpp:5:10: error: expected ';' after expression
    x = 5
         ^
         ;
1 error generated.
```

**Key Insights:**
- Show EXACTLY where to insert/remove characters
- Don't overwhelm with nested errors
- Provide "verbose" mode for experts
- Minimize template/macro noise

---

### 5. **Swift** - Apple's Modern Take

**Philosophy:** "Clear and consistent"
- Emoji indicators (⚠️ ✋ 💡)
- Consistent format across all errors
- "Note:" for additional context
- Playground-style inline feedback

**Key Insights:**
- Visual indicators help quick scanning
- Consistency reduces cognitive load
- Inline feedback (IDE integration) is powerful

---

## Windjammer Error Message Principles

### Core Philosophy: **"Empower, Don't Frustrate"**

The compiler should be a **helpful colleague**, not a gatekeep

er.

### The Windjammer Way: Error Message Design

1. **Clarity Over Brevity**
   - A 5-line helpful error beats a 1-line cryptic one
   - Explain WHY, not just WHAT

2. **Context Is King**
   - Show source code with highlighting
   - Show related definitions
   - Show the chain of reasoning

3. **Actionable, Not Pedantic**
   - ALWAYS suggest a fix when possible
   - Show code snippets, not just descriptions
   - "Try this:" > "You should..."

4. **Teach, Don't Scold**
   - Assume user made a reasonable mistake
   - Explain the language feature being violated
   - Link to documentation/examples

5. **Progressive Detail**
   - Level 1: Quick error (1-2 lines)
   - Level 2: Detailed explanation (default, 5-10 lines)
   - Level 3: Verbose mode (full reasoning, traces)

6. **Consistency**
   - All errors follow same format
   - Same color scheme throughout
   - Predictable structure

7. **Integration Ready**
   - Machine-readable (JSON mode)
   - LSP-friendly (locations, quick fixes)
   - CI/CD friendly (exit codes, counts)

---

## Error Message Anatomy

```
error[WXXX]: <SUMMARY>
  --> <FILE>:<LINE>:<COL>
   |
<LINE_NUM> | <SOURCE CODE>
   |        <HIGHLIGHT WITH ^^^^>
   |        <ANNOTATION>
   |
note: <ADDITIONAL CONTEXT>
   |
help: <SUGGESTED FIX>
   |
<LINE_NUM> | <FIXED CODE>
   |        <DIFF MARKERS>
   |
hint: <LEARNING OPPORTUNITY>
   |
docs: https://windjammer.dev/errors/WXXX
```

---

## Error Categories & Codes

### **W0xxx - Parse Errors**
- W0001: Unexpected token
- W0002: Missing semicolon
- W0003: Unclosed delimiter
- W0004: Invalid syntax

### **W1xxx - Type Errors**
- W1001: Type mismatch
- W1002: Unknown type
- W1003: Trait not implemented
- W1004: Type inference failure

### **W2xxx - Ownership Errors**
- W2001: Use after move
- W2002: Borrow conflict
- W2003: Lifetime mismatch
- W2004: Dangling reference

### **W3xxx - Mutability Errors**
- W3001: Cannot mutate immutable variable
- W3002: Cannot move out of mutable reference
- W3003: Mutability mismatch

### **W4xxx - Name Resolution Errors**
- W4001: Undefined variable
- W4002: Undefined function
- W4003: Undefined type
- W4004: Ambiguous name

### **W5xxx - Pattern Matching Errors**
- W5001: Non-exhaustive match
- W5002: Unreachable pattern
- W5003: Pattern type mismatch

### **W6xxx - Module System Errors**
- W6001: Module not found
- W6002: Circular dependency
- W6003: Private access

---

## Implementation Strategy

### Phase 1: Infrastructure (Week 1)
- [ ] Create `ErrorMessage` struct with all metadata
- [ ] Implement color-coded output
- [ ] Add source code highlighting
- [ ] Create error code registry

### Phase 2: Core Errors (Week 2-3)
- [ ] Parse errors with suggestions
- [ ] Type errors with constraint explanations
- [ ] Ownership errors with visual diagrams
- [ ] Mutability errors with fix suggestions

### Phase 3: Intelligence (Week 4)
- [ ] "Did you mean?" fuzzy matching
- [ ] Contextual help based on error pattern
- [ ] Learning hints (beginner-friendly)
- [ ] Quick fixes for LSP

### Phase 4: Polish (Week 5)
- [ ] Documentation for all error codes
- [ ] Examples for each error
- [ ] Error statistics tracking
- [ ] User feedback mechanism

---

## TDD Test Categories

### 1. **Parse Errors**
```rust
#[test]
fn test_missing_semicolon_suggestion() {
    let code = r#"
        fn main() {
            let x = 5
            let y = 10
        }
    "#;
    let error = compile_and_get_error(code);
    assert!(error.contains("Did you forget a semicolon?"));
    assert!(error.contains("let x = 5;"));
}
```

### 2. **Type Errors**
```rust
#[test]
fn test_type_mismatch_with_context() {
    let code = r#"
        fn add(a: i32, b: i32) -> i32 { a + b }
        fn main() {
            let result = add(5, "hello");
        }
    "#;
    let error = compile_and_get_error(code);
    assert!(error.contains("expected `i32`, found `string`"));
    assert!(error.contains("function `add` requires"));
}
```

### 3. **Ownership Errors**
```rust
#[test]
fn test_use_after_move_explanation() {
    let code = r#"
        fn main() {
            let s = String::from("hello");
            let s2 = s;
            println(s);  // Error: s was moved
        }
    "#;
    let error = compile_and_get_error(code);
    assert!(error.contains("value moved here"));
    assert!(error.contains("value used after move"));
    assert!(error.contains("consider cloning"));
}
```

### 4. **Mutability Errors**
```rust
#[test]
fn test_immutable_mutation_with_fix() {
    let code = r#"
        fn main() {
            let x = 5;
            x = 10;
        }
    "#;
    let error = compile_and_get_error(code);
    assert!(error.contains("cannot mutate immutable variable"));
    assert!(error.contains("let mut x = 5"));
}
```

### 5. **Name Resolution Errors**
```rust
#[test]
fn test_undefined_with_did_you_mean() {
    let code = r#"
        fn main() {
            let count = 5;
            println(cont);  // Typo: cont instead of count
        }
    "#;
    let error = compile_and_get_error(code);
    assert!(error.contains("undefined variable `cont`"));
    assert!(error.contains("Did you mean `count`?"));
}
```

---

## Success Metrics

### Quantitative
- **Error Resolution Time**: How fast can users fix errors?
- **Error Repeat Rate**: Do same errors occur multiple times?
- **Documentation Clicks**: Are users clicking error code links?
- **Quick Fix Usage**: Are LSP quick fixes being used?

### Qualitative
- **User Feedback**: Survey/GitHub issues about error quality
- **Beginner Success**: Can new users understand errors?
- **Teaching Effectiveness**: Do errors help users learn?

---

## Next Steps

1. ✅ Research complete
2. ⏳ Create test suite (50+ error scenarios)
3. ⏳ Implement `ErrorMessage` infrastructure
4. ⏳ Improve parse errors first (lowest hanging fruit)
5. ⏳ Iterate based on dogfooding feedback

---

**Remember:** Great error messages are the difference between a frustrating language and a delightful one. Invest the time to get this right.













