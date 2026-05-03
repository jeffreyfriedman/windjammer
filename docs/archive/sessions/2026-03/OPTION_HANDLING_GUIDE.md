# Option Handling in Windjammer

## The Problem with `.unwrap()`

`.unwrap()` panics if the `Option` is `None`, crashing your program:

```windjammer
fn get_value(data: Option<int>) -> int {
    data.unwrap()  // ⚠️ PANICS if data is None!
}
```

**Use `.unwrap()` only for:**
- Prototyping/debugging
- Tests where panic is acceptable
- Cases where `None` is truly impossible (and you can prove it)

## Idiomatic Alternatives

### 1. Pattern Matching with `if let` (Recommended)

**Best for:** Providing a default value or alternative logic

```windjammer
fn get_sum(node: Node) -> int {
    if let Some(children) = node.children {
        children[0] + children[1]
    } else {
        0  // Safe default
    }
}
```

### 2. Pattern Matching with `match`

**Best for:** Multiple branches or complex logic

```windjammer
fn process_data(data: Option<string>) -> string {
    match data {
        Some(value) => "Got: " + &value,
        None => "No data available"
    }
}
```

### 3. `unwrap_or()` - Provide Default

**Best for:** Simple default values

```windjammer
fn get_count(maybe_count: Option<int>) -> int {
    maybe_count.unwrap_or(0)  // Returns 0 if None
}
```

### 4. `unwrap_or_else()` - Computed Default

**Best for:** Expensive defaults that should only compute if needed

```windjammer
fn get_config(maybe_config: Option<Config>) -> Config {
    maybe_config.unwrap_or_else(|| Config::load_default())
}
```

### 5. `?` Operator - Propagate Errors

**Best for:** Functions that can return `Option` or `Result`

```windjammer
fn get_first_child(node: Node) -> Option<Node> {
    let children = node.children?  // Returns None if children is None
    children.first()
}
```

### 6. `map()` - Transform if Present

**Best for:** Applying operations only if value exists

```windjammer
fn double_if_present(value: Option<int>) -> Option<int> {
    value.map(|x| x * 2)
}
```

### 7. `and_then()` - Chain Operations

**Best for:** Multiple operations that can each return None

```windjammer
fn get_nested_value(data: Option<Container>) -> Option<int> {
    data.and_then(|c| c.inner)
        .and_then(|i| i.value)
}
```

## Real-World Examples

### ❌ Bad: Using `.unwrap()`

```windjammer
fn calculate_average(scores: Vec<int>) -> float {
    let total = scores.iter().sum()
    let count = scores.len()
    total / count  // Panics if scores is empty!
}
```

### ✅ Good: Returning `Option`

```windjammer
fn calculate_average(scores: Vec<int>) -> Option<float> {
    if scores.is_empty() {
        return None
    }
    let total = scores.iter().sum()
    let count = scores.len()
    Some(total as float / count as float)
}
```

### ❌ Bad: Nested `.unwrap()`

```windjammer
fn get_user_email(user_id: int) -> string {
    let user = database.find_user(user_id).unwrap()
    let profile = user.profile.unwrap()
    profile.email.unwrap()  // Triple panic risk!
}
```

### ✅ Good: Using `?` Operator

```windjammer
fn get_user_email(user_id: int) -> Option<string> {
    let user = database.find_user(user_id)?
    let profile = user.profile?
    profile.email
}
```

## Compiler Support

The Windjammer compiler automatically handles ownership for all Option methods:

```windjammer
fn process(node: Node) -> int {
    // Compiler inserts .clone() when needed:
    node.data.unwrap_or(42)        // → node.data.clone().unwrap_or(42)
    node.value.map(|x| x * 2)      // → node.value.clone().map(...)
    node.children.unwrap_or_else(|| vec![])  // → .clone().unwrap_or_else(...)
}
```

**You write natural code, the compiler handles ownership.**

## When `.unwrap()` is OK

### In Tests

```windjammer
#[test]
fn test_parser() {
    let result = parse("valid input")
    let ast = result.unwrap()  // OK: Test should panic on parse failure
    assert_eq(ast.type, ASTType::Valid)
}
```

### After Explicit Checks

```windjammer
fn process(data: Option<int>) {
    if data.is_some() {
        let value = data.unwrap()  // OK: We just checked
        println("Value: {}", value)
    }
}
```

**But even then, `if let` is cleaner:**

```windjammer
fn process(data: Option<int>) {
    if let Some(value) = data {
        println("Value: {}", value)  // Better!
    }
}
```

## Summary

**Never use `.unwrap()` in production code unless:**
1. You're in a test
2. You have an explicit `is_some()` check immediately before
3. You're 100% certain `None` is impossible (and document why)

**Prefer:**
- `if let Some(x) = ...` for simple cases
- `?` operator for error propagation
- `.unwrap_or()` / `.unwrap_or_else()` for defaults
- `.map()` / `.and_then()` for transformations

**The Windjammer Way**: Handle errors gracefully. Your users will thank you.
