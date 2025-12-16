# Compiler Plugin System Design

**Status:** Design Document (Not Implemented)  
**Date:** December 2025  
**Purpose:** Architecture for application-specific codegen without polluting core compiler

---

## Problem Statement

The Windjammer compiler currently contains **application-specific code** that violates our core philosophy:

### Current Issues

**❌ Application-Specific Code in Core Compiler:**
- **Tauri Integration** (~70 lines)
  - `is_tauri_function()` - Hardcoded function names
  - `generate_tauri_invoke()` - WASM-specific codegen
  - Tauri import handling
  - WASM helper generation
- **UI Framework Code** (removed in Phase 1, but pattern exists)

**❌ Why This Is Wrong:**
1. **Philosophy Violation**: "Windjammer is a general-purpose programming language"
2. **Coupling**: Core compiler tied to specific applications (windjammer-ui editor)
3. **Maintenance Burden**: Framework changes require compiler changes
4. **Scalability**: Can't support other frameworks without bloating compiler
5. **Testing Complexity**: Application-specific code mixed with core logic

---

## Design Philosophy

### Core Principles

1. **Separation of Concerns**
   - Core compiler: General-purpose language features
   - Plugins: Application/framework-specific codegen

2. **Zero Core Impact**
   - Plugins should not affect core compiler performance
   - Core tests should pass without any plugins loaded

3. **Discoverability**
   - Plugins should be easy to create and use
   - Clear API boundaries

4. **Composability**
   - Multiple plugins can coexist
   - Plugin ordering and priority

---

## Architecture Overview

### High-Level Design

```
┌─────────────────────────────────────────────┐
│         Windjammer Compiler Core            │
│                                             │
│  ┌──────────────────────────────────────┐  │
│  │   Parser → Analyzer → CodeGenerator  │  │
│  └──────────────────────────────────────┘  │
│                     │                       │
│                     ▼                       │
│           Plugin Hook Points                │
│    ┌──────────┬──────────┬──────────┐     │
│    │  Parse   │  Analyze │ Generate │     │
│    │  Hooks   │  Hooks   │  Hooks   │     │
│    └──────────┴──────────┴──────────┘     │
└─────────────────┬───────────────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
   ┌────▼────┐        ┌────▼────┐
   │ Tauri   │        │ Custom  │
   │ Plugin  │        │ Plugins │
   └─────────┘        └─────────┘
```

### Plugin Lifecycle

1. **Discovery**: Compiler discovers plugins (config file or CLI flags)
2. **Registration**: Plugins register for specific hook points
3. **Execution**: Compiler invokes plugins at appropriate phases
4. **Aggregation**: Compiler merges plugin outputs with core generation

---

## Plugin API Design

### Plugin Trait

```rust
// Core plugin trait that all plugins implement
pub trait CompilerPlugin: Send + Sync {
    /// Plugin metadata
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    
    /// Hook registration
    fn register_hooks(&self, registry: &mut HookRegistry);
}

/// Hook types
pub enum Hook {
    /// Parse-time hooks (modify AST after parsing)
    PostParse(Box<dyn ParseHook>),
    
    /// Analysis hooks (add custom analysis)
    PreAnalyze(Box<dyn AnalysisHook>),
    PostAnalyze(Box<dyn AnalysisHook>),
    
    /// Code generation hooks
    PreCodegen(Box<dyn CodegenHook>),
    FunctionCall(Box<dyn FunctionCallHook>),
    Import(Box<dyn ImportHook>),
    PostCodegen(Box<dyn CodegenHook>),
}
```

### Hook Interfaces

```rust
/// Function call interception
pub trait FunctionCallHook: Send + Sync {
    /// Check if this plugin handles the function call
    fn handles(&self, function_name: &str, context: &CallContext) -> bool;
    
    /// Generate custom code for the function call
    fn generate(&self, 
                function_name: &str, 
                arguments: &[Expression],
                context: &CallContext) -> Result<String, PluginError>;
}

/// Import handling
pub trait ImportHook: Send + Sync {
    /// Check if this plugin handles the import
    fn handles(&self, module_path: &[String]) -> bool;
    
    /// Generate custom import code
    fn generate(&self, module_path: &[String]) -> Result<String, PluginError>;
    
    /// Skip default import generation?
    fn skip_default(&self) -> bool { false }
}

/// Custom preamble/prologue generation
pub trait CodegenHook: Send + Sync {
    /// Generate additional code at the start/end of compilation unit
    fn generate(&self, context: &CodegenContext) -> Result<String, PluginError>;
}
```

### Context Objects

```rust
/// Context provided to function call hooks
pub struct CallContext {
    pub target: CompilationTarget,  // Native, WASM, etc.
    pub current_function: Option<String>,
    pub module_path: Vec<String>,
    pub is_async: bool,
}

/// Context for codegen hooks
pub struct CodegenContext {
    pub target: CompilationTarget,
    pub module_name: String,
    pub imports: Vec<UseDeclaration>,
}
```

---

## Example Plugin: Tauri

### Implementation

```rust
pub struct TauriPlugin {
    tauri_functions: HashSet<String>,
}

impl TauriPlugin {
    pub fn new() -> Self {
        let mut functions = HashSet::new();
        functions.insert("read_file".to_string());
        functions.insert("write_file".to_string());
        functions.insert("list_directory".to_string());
        // ... other Tauri commands
        
        Self {
            tauri_functions: functions,
        }
    }
}

impl CompilerPlugin for TauriPlugin {
    fn name(&self) -> &str { "tauri" }
    fn version(&self) -> &str { "1.0.0" }
    fn description(&self) -> &str { 
        "Tauri framework integration for WASM compilation"
    }
    
    fn register_hooks(&self, registry: &mut HookRegistry) {
        registry.register(Hook::FunctionCall(
            Box::new(TauriFunctionCallHook::new(self.tauri_functions.clone()))
        ));
        registry.register(Hook::Import(
            Box::new(TauriImportHook)
        ));
        registry.register(Hook::PreCodegen(
            Box::new(TauriPreambleHook)
        ));
    }
}

struct TauriFunctionCallHook {
    tauri_functions: HashSet<String>,
}

impl FunctionCallHook for TauriFunctionCallHook {
    fn handles(&self, function_name: &str, context: &CallContext) -> bool {
        // Only handle for WASM target
        context.target == CompilationTarget::Wasm 
            && self.tauri_functions.contains(function_name)
    }
    
    fn generate(&self, 
                function_name: &str, 
                arguments: &[Expression],
                _context: &CallContext) -> Result<String, PluginError> {
        // Generate tauri::invoke() call
        let args = generate_args_object(arguments)?;
        Ok(format!(
            "tauri_invoke::<_>(\"{}\", {})",
            function_name,
            args
        ))
    }
}

struct TauriImportHook;

impl ImportHook for TauriImportHook {
    fn handles(&self, module_path: &[String]) -> bool {
        module_path.first().map(|s| s.as_str()) == Some("tauri")
    }
    
    fn generate(&self, _module_path: &[String]) -> Result<String, PluginError> {
        // Skip - Tauri functions are generated inline
        Ok(String::new())
    }
    
    fn skip_default(&self) -> bool { true }
}

struct TauriPreambleHook;

impl CodegenHook for TauriPreambleHook {
    fn generate(&self, context: &CodegenContext) -> Result<String, PluginError> {
        if context.target != CompilationTarget::Wasm {
            return Ok(String::new());
        }
        
        Ok(r#"
// Tauri invoke helper for WASM
#[wasm_bindgen(inline_js = "...")]
extern "C" {
    async fn tauri_invoke_js(cmd: &str, args: JsValue) -> JsValue;
}

async fn tauri_invoke<T: serde::de::DeserializeOwned>(
    cmd: &str, 
    args: serde_json::Value
) -> Result<T, String> {
    // Implementation...
}
"#.to_string())
    }
}
```

### Usage

```rust
// In compiler initialization
let mut compiler = WjCompiler::new();

// Register plugins
if config.enable_tauri_plugin {
    compiler.register_plugin(Box::new(TauriPlugin::new()));
}

// Compile
compiler.compile(&source)?;
```

---

## Plugin Discovery

### Configuration File

```toml
# windjammer.toml
[plugins]
enabled = ["tauri", "custom-backend"]

[plugins.tauri]
version = "1.0.0"
enable_for_targets = ["wasm"]

[plugins.custom-backend]
path = "./plugins/custom_backend.wasm"  # WASM-based plugin
```

### CLI Flags

```bash
# Enable specific plugin
windjammer compile --plugin tauri main.wj

# Disable all plugins (core-only mode)
windjammer compile --no-plugins main.wj

# List available plugins
windjammer plugins list
```

---

## Plugin Distribution

### Options

1. **Built-in Plugins** (shipped with compiler)
   - Tauri
   - Common frameworks

2. **Dynamic Loading** (future)
   - Load `.so`/`.dylib`/`.dll` at runtime
   - WASM-based plugins (portable, sandboxed)

3. **Source-based Plugins** (simplest)
   - Rust crates that implement `CompilerPlugin`
   - Compiled into custom compiler binary

---

## Implementation Phases

### Phase 1: Core Infrastructure (2-4 weeks)
- [ ] Define plugin traits and hook interfaces
- [ ] Implement `HookRegistry` and plugin loading
- [ ] Add hook points to code generator
- [ ] Write plugin system tests

### Phase 2: Tauri Migration (1 week)
- [ ] Implement `TauriPlugin`
- [ ] Remove Tauri code from core generator
- [ ] Verify WASM compilation still works
- [ ] Update tests

### Phase 3: Documentation & Examples (1 week)
- [ ] Plugin development guide
- [ ] Example custom plugins
- [ ] Migration guide for existing code

### Phase 4: Advanced Features (future)
- [ ] Dynamic plugin loading (WASM or native)
- [ ] Plugin marketplace/registry
- [ ] Hot-reload during development
- [ ] Plugin debugging tools

---

## Benefits

### For Core Compiler
✅ **Cleaner codebase**: No application-specific code  
✅ **Better testability**: Test core and plugins separately  
✅ **Faster compilation**: Load only needed plugins  
✅ **Philosophy alignment**: True general-purpose language

### For Plugin Developers
✅ **Independence**: Update plugin without compiler changes  
✅ **Flexibility**: Custom codegen for any framework  
✅ **Composability**: Combine multiple plugins  
✅ **Safety**: Sandboxed execution (WASM plugins)

### For Users
✅ **Choice**: Use only plugins you need  
✅ **Performance**: Smaller compiler binary  
✅ **Ecosystem**: Community plugins  
✅ **Stability**: Core compiler more stable

---

## Alternative Approaches Considered

### 1. Macro System
**Pros:** Flexible, user-programmable  
**Cons:** Complex, security risks, harder to optimize  
**Decision:** Plugin system simpler for framework integration

### 2. External Code Generators
**Pros:** Complete separation  
**Cons:** Multiple compilation passes, integration complexity  
**Decision:** Plugins better integrated with compiler

### 3. Configuration Files
**Pros:** Simple, no code  
**Cons:** Limited flexibility, can't handle complex cases  
**Decision:** Config good for simple cases, plugins for complex

---

## Security Considerations

### Plugin Sandboxing

1. **WASM-based Plugins**
   - Run in sandboxed environment
   - Limited system access
   - Cannot corrupt compiler state

2. **Capability-based Security**
   - Plugins declare required capabilities
   - User approves before first use
   - Revocable permissions

3. **Code Signing**
   - Official plugins signed by Windjammer team
   - Community plugins verified by maintainers
   - Unsigned plugins require explicit approval

---

## Migration Strategy

### Step 1: Core Infrastructure
Implement plugin system without removing existing code

### Step 2: Parallel Implementation
Implement Tauri as plugin alongside existing code

### Step 3: Validation
Verify plugin produces identical output

### Step 4: Removal
Remove original Tauri code from core

### Step 5: Documentation
Update all docs to reference plugin system

---

## Success Metrics

✅ **Core Compiler Size**: Reduce by ~100 lines (Tauri removal)  
✅ **Compilation Speed**: No regression with 0 plugins loaded  
✅ **Plugin Performance**: <5% overhead with 1 plugin  
✅ **Developer Experience**: Plugin creation < 100 lines for simple cases  
✅ **Philosophy Compliance**: Zero application-specific code in core

---

## Future Extensions

### Custom Backends
Plugins could generate code for:
- Different programming languages (Python, C++)
- Custom VMs
- Hardware description languages

### Language Extensions
Plugins could add:
- Custom syntax (via parse hooks)
- New type system features
- Domain-specific optimizations

### IDE Integration
Plugins provide:
- Custom diagnostics
- Code completion
- Refactoring support

---

## Conclusion

The compiler plugin system will enable Windjammer to remain a **pure, general-purpose programming language** while supporting diverse application domains through opt-in plugins.

**Key Principles:**
- Core compiler stays clean and fast
- Application-specific code lives in plugins
- Users choose only plugins they need
- Plugin ecosystem can grow independently

**Next Steps:**
1. Review and approve this design
2. Implement Phase 1 (core infrastructure)
3. Migrate Tauri code to plugin
4. Open-source plugin API for community

---

**"The compiler should be simple. Complexity belongs in plugins."**

