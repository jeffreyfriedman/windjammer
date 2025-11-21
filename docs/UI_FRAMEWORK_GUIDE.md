# Windjammer UI Framework Guide

## Introduction

Windjammer UI is a reactive component framework that compiles to WebAssembly with **zero JavaScript**. It combines the simplicity of Svelte with the performance of Rust, giving you 80% of React's power with 20% of the complexity.

## Quick Start

### 1. Create a Component

Create a file `counter.wj`:

```windjammer
count: int = 0

fn increment() {
    count = count + 1
}

view {
    button(on_click: increment) {
        "Count: {count}"
    }
}
```

### 2. Compile

```bash
wj build counter.wj --target wasm --output ./counter_output
```

### 3. Build and Run

```bash
cd counter_output
wasm-pack build --target web

# Serve with Windjammer dev server
wj run ../examples/dev_server.wj
```

Open `http://localhost:8080` and see your component in action!

## Component Syntax

Windjammer supports two syntax styles:

### Minimal Syntax (Recommended)

The minimal syntax is inspired by Svelte and emphasizes simplicity:

```windjammer
// State - automatically reactive
count: int = 0
name: String = "World"

// Computed values - automatically recalculated
@computed
doubled: int = count * 2

// Functions - event handlers and helpers
fn increment() {
    count = count + 1
}

// View - JSX-like template syntax
view {
    div {
        "Hello, {name}!"
        button(on_click: increment) {
            "Count: {count}"
        }
        "Doubled: {doubled}"
    }
}
```

### Advanced Syntax (Escape Hatch)

For more control, use the advanced syntax:

```windjammer
@component
struct Counter {
    count: int = 0
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }
    
    fn increment(&mut self) {
        self.count += 1
    }
    
    fn render(&self) -> VNode {
        // Manual VNode construction
    }
}
```

## State Management

### Reactive Variables

All top-level variables are automatically reactive:

```windjammer
count: int = 0
name: String = "Alice"
items: Vec<String> = vec!["a", "b", "c"]
```

Under the hood, these become `Signal<T>` which automatically track dependencies and trigger updates.

### Computed Values

Computed values are derived from reactive state:

```windjammer
count: int = 0

@computed
doubled: int = count * 2

@computed
is_even: bool = count % 2 == 0
```

Computed values:
- Automatically recalculate when dependencies change
- Are cached until dependencies change
- Can depend on other computed values

### Mutable State

All state is mutable by default:

```windjammer
count: int = 0

fn increment() {
    count = count + 1  // Automatically calls count.set(count.get() + 1)
}
```

## View Syntax

### Elements

Create HTML elements with a simple syntax:

```windjammer
view {
    div {
        h1 { "Title" }
        p { "Paragraph" }
    }
}
```

### Attributes

#### Static Attributes

```windjammer
div(class: "container", id: "main") {
    "Content"
}
```

#### Dynamic Attributes

```windjammer
active: bool = true

view {
    button(class: if active { "active" } else { "inactive" }) {
        "Click me"
    }
}
```

#### Event Handlers

Event handlers use the `on_*` prefix:

```windjammer
fn handle_click() {
    // ...
}

fn handle_input(event: Event) {
    // ...
}

view {
    button(on_click: handle_click) { "Click" }
    input(on_input: handle_input, on_keypress: handle_keypress)
}
```

### Text Interpolation

Use `{variable}` to interpolate values:

```windjammer
name: String = "World"
count: int = 42

view {
    div {
        "Hello, {name}!"
        "Count: {count}"
    }
}
```

### Conditionals

Use `if` and `else` for conditional rendering:

```windjammer
show: bool = true

view {
    if show {
        div { "Visible" }
    } else {
        div { "Hidden" }
    }
}
```

### Lists

Use `for` loops to render lists:

```windjammer
items: Vec<String> = vec!["Apple", "Banana", "Cherry"]

view {
    ul {
        for item in items {
            li { "{item}" }
        }
    }
}
```

With indices:

```windjammer
for (index, item) in items.iter().enumerate() {
    li {
        "{index}: {item}"
    }
}
```

### Component Composition

Use other components:

```windjammer
view {
    div {
        Button(text: "Click me", on_click: handler)
        Card(title: "Hello") {
            "Card content"
        }
    }
}
```

## Lifecycle Hooks

### @on_mount

Called when the component is first mounted:

```windjammer
@on_mount
fn setup() {
    // Initialize data, set up subscriptions, etc.
}
```

### @on_destroy

Called when the component is unmounted:

```windjammer
@on_destroy
fn cleanup() {
    // Clean up subscriptions, timers, etc.
}
```

### @on_update

Called after the component updates:

```windjammer
@on_update
fn after_update() {
    // React to state changes
}
```

## Advanced Features

### Props (Coming Soon)

Pass data to components:

```windjammer
@component
struct Button {
    text: String
    on_click: fn()
}
```

### Slots (Coming Soon)

Allow children to be passed to components:

```windjammer
view {
    Card {
        "This content goes in the card"
    }
}
```

### Context (Coming Soon)

Share data across component trees:

```windjammer
@context
theme: Theme = Theme::Light
```

## Best Practices

### 1. Keep Components Small

Break large components into smaller, reusable pieces:

```windjammer
// Good
view {
    Header()
    MainContent()
    Footer()
}

// Avoid
view {
    div {
        // 500 lines of nested elements...
    }
}
```

### 2. Use Computed Values

Don't recalculate values in the view:

```windjammer
// Good
@computed
filtered_items: Vec<String> = items.filter(|item| item.len() > 3)

view {
    for item in filtered_items {
        div { "{item}" }
    }
}

// Avoid
view {
    for item in items.filter(|item| item.len() > 3) {
        div { "{item}" }
    }
}
```

### 3. Name Event Handlers Clearly

```windjammer
// Good
fn handle_submit() { }
fn handle_input_change() { }
fn handle_delete_click() { }

// Avoid
fn do_thing() { }
fn handler1() { }
fn f() { }
```

### 4. Group Related State

```windjammer
// Good
user_name: String = ""
user_email: String = ""
user_age: int = 0

// Or use a struct
user: User = User::new()
```

## Performance Tips

### 1. Minimize Computed Dependencies

Computed values recalculate when any dependency changes:

```windjammer
// Good - only depends on filtered_items
@computed
filtered_items: Vec<String> = items.filter(|item| item.active)

@computed
count: int = filtered_items.len()

// Avoid - recalculates count every time items changes
@computed
count: int = items.filter(|item| item.active).len()
```

### 2. Use Keys for Lists (Coming Soon)

Help the framework identify list items:

```windjammer
for item in items {
    div(key: item.id) {
        "{item.name}"
    }
}
```

### 3. Avoid Inline Functions

```windjammer
// Good
fn handle_click() {
    // ...
}

view {
    button(on_click: handle_click) { "Click" }
}

// Avoid (creates new function on every render)
view {
    button(on_click: || { /* ... */ }) { "Click" }
}
```

## Comparison to Other Frameworks

### vs React
- ✅ Simpler syntax (no JSX transpilation needed)
- ✅ No virtual DOM overhead
- ✅ Automatic reactivity (no useState/useEffect)
- ✅ Compiles to WASM (faster)
- ✅ Zero JavaScript runtime

### vs Svelte
- ✅ Similar syntax and philosophy
- ✅ Compiles to WASM instead of JavaScript
- ✅ Type-safe (Rust-based)
- ✅ Better performance for compute-heavy tasks

### vs Vue
- ✅ Simpler reactivity model
- ✅ No template vs script separation
- ✅ Compiles to WASM
- ✅ Type-safe

## Examples

### Counter

```windjammer
count: int = 0

fn increment() { count = count + 1 }
fn decrement() { count = count - 1 }
fn reset() { count = 0 }

view {
    div(class: "counter") {
        h1 { "Counter" }
        div(class: "display") { "Count: {count}" }
        div(class: "controls") {
            button(on_click: decrement) { "-" }
            button(on_click: reset) { "Reset" }
            button(on_click: increment) { "+" }
        }
    }
}
```

### TODO List

See `examples/components/todo.wj` for a complete TODO app example.

### Form Validation (Coming Soon)

```windjammer
email: String = ""
password: String = ""

@computed
is_valid: bool = email.contains("@") && password.len() >= 8

fn submit() {
    if is_valid {
        // Submit form
    }
}

view {
    form(on_submit: submit) {
        input(type: "email", value: email, on_input: update_email)
        input(type: "password", value: password, on_input: update_password)
        button(disabled: !is_valid) { "Submit" }
    }
}
```

## Troubleshooting

### Component Not Updating

Make sure you're modifying reactive state:

```windjammer
// This works
count = count + 1

// This doesn't (direct mutation)
items[0] = "new value"  // Use items.set(0, "new value") instead
```

### Build Errors

1. Check syntax - make sure braces match
2. Verify types - Windjammer is type-safe
3. Check function signatures
4. Run `wj check` to see detailed errors

### Performance Issues

1. Use computed values for expensive calculations
2. Minimize dependencies in computed values
3. Profile with browser dev tools
4. Consider code splitting for large apps

## Next Steps

- Read the [API Reference](./API_REFERENCE.md)
- Explore [Examples](../examples/components/)
- Join the [Community](https://github.com/jeffreyfriedman/windjammer)
- Contribute to [Windjammer](https://github.com/jeffreyfriedman/windjammer)

## Conclusion

Windjammer UI brings the best of modern web frameworks to WebAssembly, giving you:
- **Simple** syntax inspired by Svelte
- **Fast** performance with WASM
- **Type-safe** with Rust
- **Zero** JavaScript runtime

Start building reactive web apps today with Windjammer UI!
