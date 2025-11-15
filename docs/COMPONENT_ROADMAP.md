# Windjammer UI Component Roadmap

## Goal: Parity with shadcn/ui

Based on [shadcn/ui components](https://ui.shadcn.com/docs/components), we need to expand our component library while maintaining the **Windjammer philosophy**:

- ✅ **Pure Windjammer API** - Developers write only Windjammer code
- ✅ **Type-safe** - Compile-time checks prevent errors
- ✅ **Reactive** - Automatic UI updates with `Signal<T>`
- ✅ **Composable** - Components nest naturally via `ToVNode`
- ✅ **Cross-platform** - Same code for browser and desktop

---

## Current Components (14)

| Component | Status | Notes |
|-----------|--------|-------|
| Button | ✅ Complete | Variants, sizes, events |
| Text | ✅ Complete | Sizes, colors |
| Panel | ✅ Complete | Containers with titles |
| Flex | ✅ Complete | Flexbox layouts |
| Container | ✅ Complete | Basic containers |
| Input | ✅ Complete | Text inputs |
| CodeEditor | ✅ Complete | Syntax highlighting |
| Alert | ✅ Complete | Info/warning/error |
| Card | ✅ Complete | Content cards |
| Grid | ✅ Complete | Grid layouts |
| Toolbar | ✅ Complete | Button groups |
| Tabs | ✅ Complete | Tabbed interfaces |
| FileTree | ✅ Complete | Hierarchical navigation |
| Custom | ✅ Complete | Via `ToVNode` trait |

---

## Priority Components (shadcn/ui parity)

### High Priority (Core Interactions)

| Component | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| **Checkbox** | 🔴 High | Low | Essential form control |
| **Radio Group** | 🔴 High | Low | Essential form control |
| **Select** | 🔴 High | Medium | Dropdown selection |
| **Switch** | 🔴 High | Low | Toggle control |
| **Dialog** | 🔴 High | Medium | Modal dialogs |
| **Dropdown Menu** | 🔴 High | Medium | Context menus |
| **Tooltip** | 🔴 High | Low | Hover information |
| **Slider** | 🔴 High | Medium | Range input |

### Medium Priority (Enhanced UX)

| Component | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| **Accordion** | 🟡 Medium | Low | Collapsible sections |
| **Popover** | 🟡 Medium | Medium | Floating content |
| **Progress** | 🟡 Medium | Low | Progress bars |
| **Spinner** | 🟡 Medium | Low | Loading indicator |
| **Badge** | 🟡 Medium | Low | Status indicators |
| **Avatar** | 🟡 Medium | Low | User images |
| **Breadcrumb** | 🟡 Medium | Low | Navigation trail |
| **Pagination** | 🟡 Medium | Medium | Page navigation |

### Lower Priority (Advanced)

| Component | Priority | Complexity | Notes |
|-----------|----------|------------|-------|
| **Data Table** | 🟢 Low | High | Sortable tables |
| **Calendar** | 🟢 Low | High | Date picker |
| **Date Picker** | 🟢 Low | High | Date selection |
| **Combobox** | 🟢 Low | High | Searchable select |
| **Command** | 🟢 Low | High | Command palette |
| **Context Menu** | 🟢 Low | Medium | Right-click menu |
| **Drawer** | 🟢 Low | Medium | Side panel |
| **Hover Card** | 🟢 Low | Medium | Rich hover content |
| **Menubar** | 🟢 Low | Medium | Application menu |
| **Navigation Menu** | 🟢 Low | High | Complex navigation |
| **Resizable** | 🟢 Low | High | Resizable panels |
| **Scroll Area** | 🟢 Low | Medium | Custom scrollbars |
| **Separator** | 🟢 Low | Low | Visual dividers |
| **Sheet** | 🟢 Low | Medium | Side sheets |
| **Sidebar** | 🟢 Low | High | Application sidebar |
| **Skeleton** | 🟢 Low | Low | Loading placeholders |
| **Sonner** | 🟢 Low | Medium | Toast notifications |
| **Table** | 🟢 Low | Medium | Basic tables |
| **Textarea** | 🟢 Low | Low | Multi-line input |
| **Toast** | 🟢 Low | Medium | Notifications |
| **Toggle** | 🟢 Low | Low | Toggle button |
| **Toggle Group** | 🟢 Low | Medium | Button group toggle |
| **Typography** | 🟢 Low | Low | Text styles |

---

## Implementation Plan

### Phase 1: Core Form Controls (Week 1)
Focus on essential form components for the game editor.

1. **Checkbox** - Boolean selection
   ```windjammer
   Checkbox::new("Enable sound")
       .checked(sound_enabled.get())
       .on_change(move |checked| sound_enabled.set(checked))
   ```

2. **Radio Group** - Single selection from options
   ```windjammer
   RadioGroup::new("difficulty")
       .option("easy", "Easy")
       .option("medium", "Medium")
       .option("hard", "Hard")
       .selected(difficulty.get())
       .on_change(move |value| difficulty.set(value))
   ```

3. **Select** - Dropdown selection
   ```windjammer
   Select::new()
       .option("rust", "Rust")
       .option("go", "Go")
       .option("python", "Python")
       .value(language.get())
       .on_change(move |value| language.set(value))
   ```

4. **Switch** - Toggle control
   ```windjammer
   Switch::new()
       .checked(enabled.get())
       .on_change(move |checked| enabled.set(checked))
   ```

### Phase 2: Dialogs & Menus (Week 2)
Essential for the game editor's create/open/save workflows.

5. **Dialog** - Modal dialogs
   ```windjammer
   Dialog::new()
       .title("Create New Game")
       .content(/* form content */)
       .open(dialog_open.get())
       .on_close(move || dialog_open.set(false))
   ```

6. **Dropdown Menu** - Context menus
   ```windjammer
   DropdownMenu::new()
       .trigger(Button::new("File"))
       .item("New", move || create_new())
       .item("Open", move || open_file())
       .separator()
       .item("Exit", move || exit())
   ```

7. **Tooltip** - Hover information
   ```windjammer
   Tooltip::new("Save your game")
       .child(Button::new("Save"))
   ```

### Phase 3: Enhanced UX (Week 3)
Improve the overall user experience.

8. **Slider** - Range input
9. **Progress** - Progress bars
10. **Spinner** - Loading indicators
11. **Badge** - Status indicators
12. **Accordion** - Collapsible sections

### Phase 4: Advanced Components (Week 4+)
Complex components for rich applications.

- Data tables
- Date pickers
- Command palette
- Advanced navigation

---

## Windjammer Philosophy Considerations

### 1. **Pure Windjammer API**
All components must be usable from pure Windjammer code:

```windjammer
use std::ui::*;

fn main() {
    let checked = Signal::new(false);
    
    let app = ReactiveApp::new("App", move || {
        Container::new()
            .child(Checkbox::new("Accept terms")
                .checked(checked.get())
                .on_change(move |value| checked.set(value)))
            .to_vnode()
    });
    
    app.run();
}
```

### 2. **Type Safety**
Use enums for variants, not strings:

```rust
// Good
pub enum CheckboxSize {
    Small,
    Medium,
    Large,
}

// Bad
pub fn size(mut self, size: &str) -> Self { ... }
```

### 3. **Reactive by Default**
Components should work naturally with `Signal<T>`:

```windjammer
let value = Signal::new("initial");

Input::new()
    .value(value.get())
    .on_change(move |new_value| value.set(new_value))
```

### 4. **Composable**
All components implement `ToVNode` for flexible nesting:

```rust
impl ToVNode for Checkbox {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
```

### 5. **Cross-Platform**
Components should work in both WASM (browser) and Tauri (desktop):

```rust
#[cfg(target_arch = "wasm32")]
// WASM-specific implementation

#[cfg(not(target_arch = "wasm32"))]
// Desktop-specific implementation
```

---

## Next Steps

1. ✅ **Integrate TODO app into showcase** - DONE
2. 🔄 **Implement Phase 1 components** - IN PROGRESS
   - Start with Checkbox
   - Then Radio Group
   - Then Select
   - Then Switch
3. 🔄 **Make game editor functional** - IN PROGRESS
   - Use new form components
   - Implement create/open/save
   - Add game preview

---

## References

- [shadcn/ui Components](https://ui.shadcn.com/docs/components)
- Current implementation: `crates/windjammer-ui/src/components/`
- Examples: `examples/`
- Documentation: `docs/`

