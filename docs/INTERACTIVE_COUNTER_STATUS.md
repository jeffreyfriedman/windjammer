# Interactive Counter - Current Status

## ✅ What Works

1. **Signal<T> is Cloneable** ✅
   - Added `#[derive(Clone)]` to `Signal<T>`
   - Signals can be cloned and shared across closures
   - Internal state uses `Rc<RefCell<T>>` for shared mutable access

2. **@export Functions are Public** ✅
   - Compiler now adds `pub` to functions with `@export` decorator
   - WASM bindings work correctly with `#[wasm_bindgen]`

3. **Compilation Pipeline** ✅
   - Windjammer → Rust → WASM compilation works
   - Event handlers compile successfully
   - Signal cloning works in generated code

4. **UI Rendering** ✅
   - Counter UI renders in browser
   - Buttons appear correctly
   - Layout works as expected

## ❌ What Doesn't Work Yet

### 1. **Reactive Re-rendering** ❌

**Problem**: When a `Signal` value changes, the UI doesn't update.

**Why**: The current implementation renders the UI once on mount, but doesn't set up reactivity to re-render when signals change.

**Example**:
```windjammer
let count = Signal::new(0)

// This renders "Count: 0"
Text::new(format!("Count: {}", count.get()))

// When button is clicked:
count.set(count.get() + 1)  // Signal updates
// But the UI still shows "Count: 0" ❌
```

**What's Needed**:
- Track which VNodes depend on which Signals
- When a Signal changes, mark dependent VNodes as "dirty"
- Re-render dirty VNodes and update the DOM

### 2. **Event Handlers Don't Trigger Updates** ❌

**Problem**: Button clicks execute the closure, but the UI doesn't reflect changes.

**Current Behavior**:
```javascript
// Button click handler runs
count.set(5)  // ✅ Signal value changes
console.log(count.get())  // ✅ Shows 5
// But DOM still shows old value ❌
```

**What's Needed**:
- After event handler executes, trigger a re-render
- Update only the parts of the DOM that changed (Virtual DOM diffing)

## 🔧 How to Fix: Reactive Re-rendering

### Option 1: Effect-Based Reactivity (Solid.js style)

```rust
// In Signal::get()
pub fn get(&self) -> T {
    // Track this read in the current reactive context
    REACTIVE_CONTEXT.with(|ctx| {
        if let Some(effect_id) = ctx.borrow().current_effect {
            self.subscribers.borrow_mut().insert(effect_id);
        }
    });
    self.value.borrow().clone()
}

// In Signal::set()
pub fn set(&self, value: T) {
    self.value.replace(value);
    // Notify all subscribers
    for effect_id in self.subscribers.borrow().iter() {
        EFFECT_REGISTRY.with(|registry| {
            if let Some(effect) = registry.borrow().get(effect_id) {
                effect.run();  // Re-run the effect
            }
        });
    }
}
```

### Option 2: Component-Based Reactivity (React style)

```rust
struct Component {
    signals: Vec<SignalId>,
    render_fn: Box<dyn Fn() -> VNode>,
}

impl Component {
    fn mount(&self) -> Element {
        // Subscribe to all signals used in render
        for signal_id in &self.signals {
            SIGNAL_REGISTRY.subscribe(signal_id, self.id, || {
                self.re_render();
            });
        }
        self.render()
    }
    
    fn re_render(&self) {
        let new_vnode = (self.render_fn)();
        let old_vnode = self.current_vnode.clone();
        let patches = diff(old_vnode, new_vnode);
        apply_patches(self.dom_element, patches);
    }
}
```

### Option 3: Manual Re-rendering (Simplest for Now)

```windjammer
// User explicitly calls re-render
let count = Signal::new(0)

fn render(count: Signal<i32>) -> Container {
    Container::new()
        .child(Text::new(format!("Count: {}", count.get())))
        .child(Button::new("Increment")
            .on_click(move || {
                count.set(count.get() + 1)
                App::re_render()  // Manually trigger re-render
            }))
}

App::new("Counter", render(count)).run()
```

## 📋 Implementation Plan

### Phase 1: Manual Re-rendering (2-3 hours)
1. Add `App::re_render()` method
2. Store root VNode in App
3. On re-render, diff and patch DOM
4. Update counter example to call `re_render()`

### Phase 2: Automatic Effect-Based Reactivity (4-6 hours)
1. Implement `Effect` system
2. Track Signal reads in effects
3. Auto-run effects when Signals change
4. Wrap event handlers in effects

### Phase 3: Virtual DOM Diffing (6-8 hours)
1. Implement VNode diffing algorithm
2. Generate minimal DOM patches
3. Apply patches efficiently
4. Handle keyed lists

### Phase 4: Component System (8-10 hours)
1. Define Component trait
2. Implement component lifecycle
3. Add props and state management
4. Support component composition

## 🎯 Next Steps

**Immediate** (to prove interactivity works):
1. Add console.log to button click handlers
2. Verify Signal values change
3. Document current limitations

**Short-term** (to get working counter):
1. Implement manual re-rendering
2. Update counter example
3. Test in browser

**Medium-term** (for React-like experience):
1. Implement automatic reactivity
2. Add Virtual DOM diffing
3. Create more examples (Todo app, etc.)

## 🧪 Testing Strategy

### Current Test:
```bash
cd build_counter
# Serve with HTTP server
python3 -m http.server 8000  # (User wants Windjammer server!)
# Open http://localhost:8000
# Click buttons
# Open console - should see println! output
# UI won't update yet ❌
```

### After Manual Re-rendering:
```bash
# Same as above
# Click buttons
# UI updates! ✅
```

### After Automatic Reactivity:
```bash
# Same as above
# No manual re-render calls needed
# Everything just works ✅
```

## 📊 Current Architecture

```
Windjammer Code
    ↓
Signal::new(0)  ← Creates reactive state
    ↓
Button::on_click(|| count.set(...))  ← Event handler
    ↓
count.set(5)  ← Updates Signal value ✅
    ↓
??? ← Missing: Trigger re-render ❌
    ↓
Update DOM ← Doesn't happen ❌
```

## 🎉 What We've Proven

1. ✅ Windjammer can compile to WASM
2. ✅ UI components render correctly
3. ✅ Event handlers can be attached
4. ✅ Signals can be shared across closures
5. ✅ The foundation for reactivity is solid

**Next**: Implement the missing piece - reactive re-rendering!

---

**Status**: Core infrastructure complete, reactivity layer needed
**Estimated Time to Full Interactivity**: 2-3 hours (manual), 6-8 hours (automatic)
**Blocker**: None - just needs implementation


