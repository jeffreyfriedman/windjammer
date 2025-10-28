# Windjammer UI Framework - API Reference

## Overview

The Windjammer UI Framework is a reactive, component-based UI library that compiles to WebAssembly. It features automatic reactivity, zero JavaScript, and a clean, minimal syntax inspired by Svelte.

## Component Syntax

### Basic Component Structure

```windjammer
// State declarations
count: int = 0
message: String = "Hello"

// Event handlers
fn increment() {
    count = count + 1
}

// View template
view {
    div(class: "app") {
        h1 { "${message}" }
        button(on_click: increment) { "Click me" }
    }
}
```

### State Declarations

State is declared at the top level with type annotations:

```windjammer
// Primitive types
count: int = 0
price: f64 = 9.99
active: bool = true

// Strings
name: String = "Alice"

// Collections (future)
items: Vec<String> = vec![]
```

**Automatic Reactivity**: All state is automatically wrapped in `Signal<T>` and becomes reactive.

### Functions

Functions are event handlers or helper methods:

```windjammer
fn increment() {
    count = count + 1
}

fn reset() {
    count = 0
}

fn complex_logic() {
    if count > 10 {
        count = 0
    }
}
```

### View Block

The `view` block defines the component's template:

```windjammer
view {
    div(class: "container") {
        // Static text
        h1 { "Title" }
        
        // Dynamic text with interpolation
        p { "Count: ${count}" }
        
        // Nested elements
        div(class: "controls") {
            button(on_click: increment) { "+" }
        }
    }
}
```

## Element Syntax

### Basic Elements

```windjammer
div { "Content" }
span { "Text" }
button { "Click" }
input()
```

### Attributes

#### Static Attributes

```windjammer
div(class: "container")
input(type: "text", placeholder: "Enter name")
a(href: "https://example.com")
```

#### Dynamic Attributes (Future)

```windjammer
div(class: activeClass)
input(value: username)
```

### Event Handlers

Event handlers use the `on_` prefix:

```windjammer
button(on_click: handleClick) { "Click" }
input(on_input: handleInput)
div(on_mouseover: handleHover)
```

**Supported Events**:
- `on_click` - Mouse click
- `on_input` - Input change
- `on_change` - Value change
- `on_submit` - Form submit
- `on_keypress` - Key press
- `on_keydown` - Key down
- `on_keyup` - Key up
- `on_mouseover` - Mouse over
- `on_mouseout` - Mouse out
- `on_focus` - Element focused
- `on_blur` - Element blurred

### Text Interpolation

Use `${}` for string interpolation:

```windjammer
"Count: ${count}"
"Hello, ${name}!"
"Total: ${price * quantity}"
```

### Conditional Rendering (Future)

```windjammer
if condition {
    div { "Shown when true" }
} else {
    div { "Shown when false" }
}
```

### List Rendering (Future)

```windjammer
for item in items {
    div { "${item}" }
}
```

## Reactivity System

### Signals

Signals are reactive values that automatically update the UI when changed.

**Rust API**:

```rust
use windjammer_ui::reactivity::Signal;

// Create a signal
let count = Signal::new(0);

// Read value
let value = count.get();

// Set value (triggers updates)
count.set(10);

// Update with function
count.update(|v| *v += 1);
```

**In Components**: State is automatically converted to signals.

### Effects

Effects run automatically when their dependencies change.

```rust
use windjammer_ui::reactivity::Effect;

let count = Signal::new(0);

// Effect runs immediately and on every count change
Effect::new(move || {
    console_log(&format!("Count is: {}", count.get()));
});
```

**In Components**: Effects are automatically generated for reactive text nodes.

### Computed Values (Future)

```rust
use windjammer_ui::reactivity::Computed;

let count = Signal::new(5);
let doubled = Computed::new(move || count.get() * 2);

assert_eq!(doubled.get(), 10);
```

## Component Runtime API

### DOM Manipulation

```rust
use windjammer_ui::component_runtime::*;

// Get document
let doc = document()?;

// Create element
let elem = create_element("div")?;

// Set attributes
set_attribute(&elem, "class", "container")?;

// Set text
set_text(&elem, "Hello");

// Append child
append_child(&parent, &elem)?;
```

### Class Manipulation

```rust
// Add class
add_class(&elem, "active")?;

// Remove class
remove_class(&elem, "active")?;

// Toggle class
toggle_class(&elem, "active")?;

// Check if has class
if has_class(&elem, "active") {
    // ...
}

// Set multiple classes
set_classes(&elem, &["btn", "btn-primary"])?;
```

### Style Manipulation

```rust
// Set style property
set_style(&elem, "color", "red")?;

// Get style property
let color = get_style(&elem, "color")?;
```

### Element Queries

```rust
// Get by ID
let elem = get_element_by_id("app")?;

// Query selector
if let Some(elem) = query_selector(".container")? {
    // ...
}

// Query all
let nodes = query_selector_all(".item")?;
```

### Focus Management

```rust
// Focus element
focus(&elem)?;

// Blur element
blur(&elem)?;
```

### Console Logging

```rust
console_log("Debug message");
console_error("Error message");
console_warn("Warning message");
```

### Timing

```rust
// Request animation frame
request_animation_frame(|| {
    // Animation logic
})?;

// Set timeout
set_timeout(|| {
    // Delayed logic
}, 1000)?;
```

## Compilation

### Build Command

```bash
wj build component.wj --target wasm --output ./output
```

### Generated Structure

```
output/
├── Cargo.toml          # Rust project config
├── src/
│   └── lib.rs          # Generated Rust code
├── index.html          # HTML template
└── README.md           # Usage instructions
```

### WASM Build

```bash
cd output
wasm-pack build --target web
```

### Deployment

```bash
# Serve locally with Windjammer dev server
wj run ../examples/dev_server.wj

# Or use any static file server
```

## Generated Code

### Component Struct

```rust
#[derive(Clone)]
#[wasm_bindgen]
pub struct Component {
    count: Signal<i32>,
}
```

### Mount Method

```rust
#[wasm_bindgen]
impl Component {
    pub fn mount(&mut self, parent: &Element) -> Result<(), JsValue> {
        // Create DOM elements
        let elem = document.create_element("div")?;
        
        // Create reactive text node
        let text = document.create_text_node(&format!("Count: {}", self.count.get()));
        elem.append_child(&text)?;
        
        // Create effect for automatic updates
        {
            let text_node = text.clone();
            let signal = self.count.clone();
            Effect::new(move || {
                let new_text = format!("Count: {}", signal.get());
                text_node.set_node_value(Some(&new_text));
            });
        }
        
        // Event handlers
        let closure = {
            let mut self_clone = self.clone();
            Closure::wrap(Box::new(move |_event: web_sys::Event| {
                self_clone.increment();
            }) as Box<dyn FnMut(web_sys::Event)>)
        };
        button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
        closure.forget();
        
        parent.append_child(&elem)?;
        Ok(())
    }
}
```

## Performance

### Bundle Size

- **WASM**: ~46KB (optimized)
- **JS Glue**: ~13KB
- **Total**: ~59KB

### Optimizations

- Direct DOM manipulation (no virtual DOM)
- Compiled reactivity (no runtime diffing)
- Surgical updates (only changed nodes)
- WASM performance

## Browser Compatibility

- **Chrome/Edge**: 88+
- **Firefox**: 89+
- **Safari**: 15+

**Requirements**: WebAssembly support

## Examples

### Counter

```windjammer
count: int = 0

fn increment() {
    count = count + 1
}

fn decrement() {
    count = count - 1
}

view {
    div(class: "counter") {
        h1 { "Counter" }
        p { "Count: ${count}" }
        button(on_click: decrement) { "-" }
        button(on_click: increment) { "+" }
    }
}
```

### Toggle

```windjammer
active: bool = false

fn toggle() {
    active = !active
}

view {
    div {
        button(on_click: toggle) { "Toggle" }
        p { "Status: ${if active { "Active" } else { "Inactive" }}" }
    }
}
```

## Best Practices

### 1. Keep Components Small

Break large components into smaller, focused ones.

### 2. Use Descriptive Names

```windjammer
// Good
fn handleSubmit() { }
fn validateEmail() { }

// Avoid
fn do() { }
fn x() { }
```

### 3. Minimize State

Only make values reactive if they need to update the UI.

### 4. Use Computed for Derived Values

Instead of storing derived values in state, use computed properties.

### 5. Avoid Side Effects in Render

Keep view blocks pure - no state mutations.

## Troubleshooting

### Component Won't Compile

**Issue**: Parse error
**Solution**: Check syntax, ensure proper indentation

### WASM Build Fails

**Issue**: Missing dependencies
**Solution**: Ensure `wasm-pack` is installed

### Reactivity Not Working

**Issue**: UI doesn't update
**Solution**: Ensure state is declared at top level, use proper signal access

### Events Not Firing

**Issue**: Click handlers don't work
**Solution**: Check event name (use `on_click` not `onclick`)

## Future Features

- Component composition
- Lifecycle hooks (onMount, onDestroy)
- Two-way binding
- Conditional rendering
- List rendering with keys
- Slots
- Props
- Routing
- State management
- SSR (Server-Side Rendering)

## See Also

- [Component Guide](./COMPONENT_GUIDE.md)
- [Examples](../examples/components/)
- [Main Documentation](../README.md)

