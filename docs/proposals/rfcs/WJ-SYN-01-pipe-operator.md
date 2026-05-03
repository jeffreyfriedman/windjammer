# WJ-SYN-01: Pipe Operator

**Status:** 🟡 Draft (Optional)  
**Author:** Windjammer Team  
**Date:** 2026-03-21  
**Target:** v0.60+ (If accepted)  
**Priority:** Low  
**Depends On:** None (standalone feature)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Motivation](#motivation)
3. [Proposed Syntax](#proposed-syntax)
4. [Use Cases](#use-cases)
5. [Language Comparison](#language-comparison)
6. [Implementation](#implementation)
7. [Alternatives](#alternatives)
8. [Open Questions](#open-questions)
9. [Decision Criteria](#decision-criteria)

---

## Executive Summary

**Goal:** Provide an optional ergonomic improvement for chaining function calls, particularly useful with the taint tracking system.

**Core Idea:** The pipe operator `|>` allows writing `value |> function()` instead of `function(value)`, enabling left-to-right reading of transformation chains.

**Status:** This is an **optional** syntax improvement. It is **NOT required** for security features (WJ-SEC-01, WJ-SEC-02) to work. This RFC may be **rejected** if the ergonomic benefit doesn't justify the syntax addition.

**Key Question:** Does the readability improvement of pipes justify adding new syntax to the language?

---

## Motivation

### Problem: Nested Function Calls Are Hard to Read

Consider a data transformation pipeline:

```windjammer
// Without pipes: Right-to-left reading (unnatural)
let result = process(validate(parse(sanitize(input))))
//           ^^^^^^^ This happens last
//                   ^^^^^^^^ This happens first

// Reading order: inside-out (sanitize → parse → validate → process)
// Execution order: inside-out (sanitize → parse → validate → process)
```

This is particularly problematic with taint tracking:

```windjammer
// Security pipeline: sanitize → validate → query
let user = sql.escape(
    validation.validate_email(
        req.param("email")
    )
)
db.query(user)

// Very hard to read! What's the flow?
```

### Solution: Left-to-Right Pipelines

```windjammer
// With pipes: Left-to-right reading (natural)
let result = input |> sanitize() |> parse() |> validate() |> process()
//           ^^^^^ Start here
//                                                           ^^^^^^^^^ End here

// Reading order: left-to-right (sanitize → parse → validate → process)
// Execution order: left-to-right (sanitize → parse → validate → process)
```

Security pipeline becomes clear:

```windjammer
// Security pipeline with pipes
let user = req.param("email")
    |> validation.validate_email()
    |> sql.escape()
    
db.query(user)

// Flow is obvious: param → validate → escape
```

---

## Proposed Syntax

### Basic Form

```windjammer
value |> function()
```

**Desugars to:**
```windjammer
function(value)
```

### Chaining

```windjammer
value |> f() |> g() |> h()
```

**Desugars to:**
```windjammer
h(g(f(value)))
```

### With Additional Arguments

```windjammer
value |> function(arg1, arg2)
```

**Desugars to:**
```windjammer
function(value, arg1, arg2)
```

**The piped value is always inserted as the FIRST argument.**

### Method Calls

```windjammer
value |> obj.method()
```

**Desugars to:**
```windjammer
obj.method(value)
```

### Multiline Pipelines

```windjammer
let result = input
    |> step1()
    |> step2()
    |> step3()
    
// Equivalent to:
let result = step3(step2(step1(input)))
```

---

## Use Cases

### Use Case 1: Security Pipelines (Primary Motivation)

**Without pipes:**
```windjammer
fn handle_login(req: Request, db: Connection) -> Response {
    let email = sql.escape(validation.validate_email(req.param("email")))
    let password = sql.escape(validation.validate_password(req.param("password")))
    
    let user = db.execute(
        "SELECT * FROM users WHERE email = ? AND password_hash = ?",
        [email, hash(password)]
    )?
    
    Response::ok()
}
```

**With pipes:**
```windjammer
fn handle_login(req: Request, db: Connection) -> Response {
    let email = req.param("email")
        |> validation.validate_email()
        |> sql.escape()
        
    let password = req.param("password")
        |> validation.validate_password()
        |> hash()
        |> sql.escape()
    
    let user = db.execute(
        "SELECT * FROM users WHERE email = ? AND password_hash = ?",
        [email, password]
    )?
    
    Response::ok()
}
```

**Benefit:** The data flow is immediately clear. Each line is one transformation step.

### Use Case 2: Data Transformation Chains

**Without pipes:**
```windjammer
fn process_data(raw: str) -> Result<Data, Error> {
    let parsed = json.parse(raw)?
    let validated = validate_schema(parsed)?
    let normalized = normalize_fields(validated)?
    let enriched = enrich_metadata(normalized)?
    Ok(enriched)
}
```

**With pipes:**
```windjammer
fn process_data(raw: str) -> Result<Data, Error> {
    raw
        |> json.parse()?
        |> validate_schema()?
        |> normalize_fields()?
        |> enrich_metadata()
}
```

**Benefit:** No intermediate variables needed. Clear pipeline.

### Use Case 3: String Manipulation

**Without pipes:**
```windjammer
let result = trim(to_lowercase(remove_whitespace(input)))
```

**With pipes:**
```windjammer
let result = input
    |> remove_whitespace()
    |> to_lowercase()
    |> trim()
```

### Use Case 4: Collections (If/When Windjammer Adds Iterator Chains)

**Without pipes:**
```windjammer
let result = sum(map(filter(numbers, |n| n > 0), |n| n * 2))
```

**With pipes:**
```windjammer
let result = numbers
    |> filter(|n| n > 0)
    |> map(|n| n * 2)
    |> sum()
```

**Note:** This assumes Windjammer has a functional collection API. If we use Rust-style method chains (`.iter().filter().map()`), pipes are less necessary.

---

## Language Comparison

### Languages With Pipe Operators

| Language | Syntax | Notes |
|----------|--------|-------|
| **Elixir** | `value \|> function()` | Data-first functional language |
| **F#** | `value \|> function` | Functional ML-family |
| **Hack** | `value \|> function($$)` | Placeholder syntax |
| **OCaml** | `value \|> function` | Functional |
| **Raku** | `value ==> function()` | Perl successor |
| **Unix Shell** | `cmd1 \| cmd2` | Process pipes (different semantics) |

### Elixir Example

```elixir
# Elixir (heavy pipe usage)
user_input
|> String.trim()
|> String.downcase()
|> validate_email()
|> insert_into_db()
```

### F# Example

```fsharp
// F# (functional transformation)
let result =
    input
    |> parseJson
    |> validateSchema
    |> transformData
```

### Hack Example

```hack
// Hack (explicit placeholder)
$result = $data
  |> some_func($$)
  |> other_func(42, $$)  // $$ is the piped value
```

**Windjammer's approach is most similar to Elixir and F#: implicit first argument.**

---

## Implementation

### Desugaring Strategy

The pipe operator is **pure syntactic sugar**. It desugars during parsing:

```
AST:
  PipeExpr(left, right)

Desugar to:
  CallExpr(right.callee, [left] + right.args)
```

### Parser Changes

```rust
// In parser.rs
fn parse_pipe_expr(&mut self) -> Result<Expr, Error> {
    let mut left = self.parse_call_expr()?;
    
    while self.match_token(Token::Pipe) {  // |>
        let right = self.parse_call_expr()?;
        left = Expr::Call {
            callee: right.callee,
            args: vec![left].extend(right.args),
            span: left.span.merge(right.span),
        };
    }
    
    Ok(left)
}
```

### Error Handling

**Pipe with non-function:**
```windjammer
let x = 5 |> 10
//           ^^ ERROR: Expected function call after |>, found literal
```

**Pipe with incompatible types:**
```windjammer
let result: i32 = "hello" |> to_uppercase()
//                ^^^^^^^^^^^^^^^^^^^^^^^ ERROR: to_uppercase returns str, expected i32
```

**Standard type checker handles these** - no special error handling needed.

---

## Alternatives

### Alternative 1: Method Chaining (Current Approach)

**Status Quo:** Use method chaining for collections.

```windjammer
let result = numbers
    .iter()
    .filter(|n| n > 0)
    .map(|n| n * 2)
    .collect()
```

**Pros:**
- ✅ No new syntax needed
- ✅ Works for methods (not free functions)
- ✅ Standard in Rust-like languages

**Cons:**
- ❌ Doesn't work for free functions
- ❌ Requires creating iterator adapters
- ❌ Can't mix free functions and methods easily

**Example:** Taint tracking with method chaining:

```windjammer
// Can't do this (validate_email is free function):
let email = req.param("email").validate_email().sql_escape()

// Must do this:
let email = sql.escape(validation.validate_email(req.param("email")))
```

### Alternative 2: Extension Methods

**Idea:** Allow adding methods to types via `impl` blocks.

```windjammer
impl str {
    fn validate_email(self) -> Result<Clean<str>, Error> { ... }
    fn sql_escape(self) -> Clean<str> { ... }
}

// Usage:
let email = req.param("email").validate_email()?.sql_escape()
```

**Pros:**
- ✅ Works with method chaining
- ✅ Discoverable via LSP autocomplete

**Cons:**
- ❌ Namespace pollution (every string has validate_email?)
- ❌ Unclear ownership (is this stdlib or user code?)
- ❌ Conflicts with Rust interop (Rust doesn't have extension methods in this way)

### Alternative 3: Do Nothing

**Idea:** Just use nested function calls.

```windjammer
let result = process(validate(parse(sanitize(input))))
```

**Pros:**
- ✅ No new syntax
- ✅ Works today
- ✅ Familiar to C/Rust developers

**Cons:**
- ❌ Hard to read (inside-out)
- ❌ Requires intermediate variables for clarity
- ❌ Taint chains are verbose

---

## Open Questions

### 1. Operator Precedence

**Question:** Where does `|>` fit in precedence hierarchy?

**Proposal:** Lower than function calls, higher than assignment.

```windjammer
let x = a |> f() + b |> g()
// Parses as: let x = ((a |> f()) + b) |> g()

let y = a |> f(b + c)
// Parses as: let y = a |> f((b + c))
```

### 2. Right-Hand Side Restrictions

**Question:** What can appear after `|>`?

**Options:**
- **A:** Only function calls (`value |> func()`)
- **B:** Any expression (`value |> { some_block() }`)
- **C:** Function names without parens (`value |> func` desugars to `func(value)`)

**Recommendation:** Start with A (function calls only), add C if users request it.

### 3. Placeholder Syntax (Hack-style)

**Question:** Should we support explicit placeholders for non-first arguments?

```windjammer
// Hack-style
let result = data |> func(arg1, $$, arg2)
// Desugars to: func(arg1, data, arg2)

// Without placeholder
let result = data |> func(arg1, arg2)
// Desugars to: func(data, arg1, arg2)
```

**Recommendation:** Not in Phase 1. Adds complexity. Can use lambda if needed:

```windjammer
let result = data |> (|x| func(arg1, x, arg2))
```

### 4. Async Pipelines

**Question:** How do pipes work with `async`/`await`?

```windjammer
// Does this work?
let result = input
    |> async_fetch()
    |> await
    |> async_process()
    |> await
```

**Recommendation:** Evaluate when async/await is added to Windjammer (future feature).

---

## Decision Criteria

### Reasons to Accept This RFC

1. **Improves Security Code Readability** - Taint pipelines become much clearer
2. **Familiar to Functional Programmers** - Elixir, F#, OCaml users expect this
3. **Easy to Implement** - Pure syntactic sugar, no semantic changes
4. **Optional** - Developers can ignore it if they prefer nested calls
5. **Left-to-Right Reading** - Matches natural reading order (English)

### Reasons to Reject This RFC

1. **Adds Syntax** - More to learn for beginners
2. **Method Chaining Exists** - Rust-style `.method()` chains work for many cases
3. **Not Critical** - Language works fine without it
4. **Precedence Confusion** - Might conflict with other operators
5. **Ecosystem Fragmentation** - Some code uses pipes, some doesn't (style inconsistency)

### Success Metrics

If accepted, track:

- **Adoption Rate:** % of security code using pipes vs nested calls
- **Readability Studies:** A/B testing pipe vs nested code comprehension
- **LSP Support:** Does autocomplete work well with pipes?
- **Error Messages:** Are type errors clear with pipes?

### Decision

**Proposed Path:** 

1. **Phase 1 (v0.55):** Implement taint tracking WITHOUT pipes
2. **Phase 2 (v0.56):** Collect feedback on ergonomics
3. **Phase 3 (v0.60):** If users request pipes, implement as opt-in syntax
4. **Phase 4:** Evaluate adoption and decide to keep or deprecate

**Key Question for Community:** After using taint tracking for 3-6 months, do you find nested calls painful enough to justify pipes?

---

## Example: Before vs After

### Scenario: User Registration Endpoint

**Without Pipes:**

```windjammer
fn handle_register(req: Request, db: Connection) -> Response {
    let email = sql.escape(
        validation.validate_email(
            req.param("email")
        )
    )
    
    let username = sql.escape(
        validation.validate_username(
            req.param("username")
        )
    )
    
    let password = sql.escape(
        hash_password(
            validation.validate_password(
                req.param("password")
            )
        )
    )
    
    db.execute(
        "INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)",
        [email, username, password]
    )?
    
    Response::created()
}
```

**With Pipes:**

```windjammer
fn handle_register(req: Request, db: Connection) -> Response {
    let email = req.param("email")
        |> validation.validate_email()
        |> sql.escape()
    
    let username = req.param("username")
        |> validation.validate_username()
        |> sql.escape()
    
    let password = req.param("password")
        |> validation.validate_password()
        |> hash_password()
        |> sql.escape()
    
    db.execute(
        "INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)",
        [email, username, password]
    )?
    
    Response::created()
}
```

**Verdict:** Pipes make the flow **significantly clearer**. Each variable's transformation is a vertical pipeline.

---

## References

- **Elixir Pipe Operator:** https://elixir-lang.org/getting-started/enumerables-and-streams.html#the-pipe-operator
- **F# Pipelining:** https://fsharpforfunandprofit.com/posts/function-composition/
- **Hack Pipe Operator:** https://docs.hhvm.com/hack/expressions-and-operators/pipe
- **TC39 JavaScript Pipe Proposal:** https://github.com/tc39/proposal-pipeline-operator (Stage 2)

---

## Appendix: Grammar Changes

### Formal Grammar (EBNF)

**Current:**
```ebnf
expr = call_expr | binary_expr | ...
call_expr = primary_expr "(" args ")"
```

**With Pipes:**
```ebnf
expr = pipe_expr | call_expr | binary_expr | ...
pipe_expr = call_expr ("|>" call_expr)*
call_expr = primary_expr "(" args ")"
```

### Precedence Table

| Operator | Precedence | Associativity |
|----------|-----------|---------------|
| `.` (method call) | 14 | Left |
| `()` (call) | 13 | Left |
| `!` (not) | 12 | Right |
| `*` `/` `%` | 11 | Left |
| `+` `-` | 10 | Left |
| `<<` `>>` | 9 | Left |
| `<` `>` `<=` `>=` | 8 | Left |
| `==` `!=` | 7 | Left |
| `&` | 6 | Left |
| `^` | 5 | Left |
| `\|` | 4 | Left |
| **`\|>` (pipe)** | **3** | **Left** |
| `&&` | 2 | Left |
| `\|\|` | 1 | Left |
| `=` `+=` etc | 0 | Right |

---

## Conclusion

The pipe operator is an **optional ergonomic improvement** that significantly enhances the readability of transformation chains, particularly for security code with taint tracking.

**Recommendation:** 
1. Implement taint tracking first (WJ-SEC-02)
2. Gather feedback on ergonomics
3. Implement pipes in v0.60+ if community feedback is positive
4. Measure adoption and keep/deprecate based on usage

**This RFC is intentionally lower priority than security features. Pipes are "nice to have", not "must have".**

---

*Feedback welcome on syntax, precedence, and whether this feature is worth adding to the language.*
