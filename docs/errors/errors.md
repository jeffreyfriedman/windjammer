# Windjammer Error Catalog

Version: 0.1.0

## Type Errors

### E0425 - Variable not found

The compiler cannot find a variable, function, or constant with this name in the current scope.

**Common Causes:**

- Typo in the variable name
- Variable not declared before use
- Variable is out of scope
- Module not imported

**Solutions:**

- Check the spelling of the variable name
- Declare the variable before using it: let x = 42
- Import the module: use std::collections::HashMap
- Check variable scope (declared inside a block?)

**Examples:**

#### Typo in variable name

❌ Wrong:

```windjammer
let count = 10
println!("{}", cont)  // Typo: 'cont' instead of 'count'
```

✅ Correct:

```windjammer
let count = 10
println!("{}", count)  // Fixed!
```

Fixed the typo in the variable name

#### Variable not declared

❌ Wrong:

```windjammer
println!("{}", total)  // 'total' not declared
```

✅ Correct:

```windjammer
let total = 100
println!("{}", total)  // Declared first!
```

Declared the variable before using it

---

### E0308 - Type mismatch

The compiler expected one type but found another. Types must match exactly in Windjammer.

**Common Causes:**

- Passing wrong type to function
- Assigning wrong type to variable
- Returning wrong type from function
- String vs integer confusion

**Solutions:**

- Use .parse() to convert string to number: "42".parse()
- Use .to_string() to convert number to string: 42.to_string()
- Check function signature for expected types
- Use type annotations if needed: let x: int = 42

**Examples:**

#### String to int conversion

❌ Wrong:

```windjammer
let x: int = "42"  // String, not int
```

✅ Correct:

```windjammer
let x: int = "42".parse()  // Convert to int
```

Used .parse() to convert string to integer

#### Int to string conversion

❌ Wrong:

```windjammer
let s: string = 42  // Int, not string
```

✅ Correct:

```windjammer
let s: string = 42.to_string()  // Convert to string
```

Used .to_string() to convert integer to string

---

## Ownership Errors

### E0384 - Cannot modify immutable variable

Variables in Windjammer are immutable by default. To modify a variable, declare it as mutable with 'let mut'.

**Common Causes:**

- Trying to modify immutable variable
- Forgot 'mut' keyword

**Solutions:**

- Declare variable as mutable: let mut x = 42
- Create a new variable instead of modifying

**Examples:**

#### Modifying immutable variable

❌ Wrong:

```windjammer
let x = 10
x = 20  // Error: x is immutable
```

✅ Correct:

```windjammer
let mut x = 10
x = 20  // Works! x is mutable
```

Added 'mut' keyword to make variable mutable

---

## Syntax Errors

## Module Errors

## Trait Errors

