use crate::CompilationTarget;

/// Centralized decorator classification system.
///
/// All knowledge about which decorators are valid for which backends
/// lives here, eliminating hardcoded `matches!()` checks scattered
/// across codegen files.
///
/// Decorator categories:
/// - GPU: Only valid in WGSL/shader backends (@vertex, @fragment, @compute)
/// - Framework: Handled by game framework, not emitted as attributes
/// - Backend-specific: Only valid for certain compilation targets
/// - Wrapping: Modify function body rather than emitting attributes
/// - Universal: Valid for all backends
pub struct DecoratorRegistry {
    gpu_decorators: Vec<&'static str>,
    framework_decorators: Vec<&'static str>,
    wrapping_decorators: Vec<&'static str>,
    wasm_only_decorators: Vec<&'static str>,
    internal_decorators: Vec<&'static str>,
}

impl DecoratorRegistry {
    pub fn new() -> Self {
        Self {
            gpu_decorators: vec!["vertex", "fragment", "compute"],
            framework_decorators: vec![
                "component",
                "game",
                "init",
                "update",
                "render",
                "render3d",
                "input",
                "cleanup",
            ],
            wrapping_decorators: vec![
                "timeout",
                "bench",
                "profile",
                "requires",
                "ensures",
                "property_test",
                "invariant",
            ],
            wasm_only_decorators: vec!["export"],
            internal_decorators: vec!["async"],
        }
    }

    pub fn is_gpu_decorator(&self, name: &str) -> bool {
        self.gpu_decorators.contains(&name)
    }

    pub fn is_framework_decorator(&self, name: &str) -> bool {
        self.framework_decorators.contains(&name)
    }

    pub fn is_wrapping_decorator(&self, name: &str) -> bool {
        self.wrapping_decorators.contains(&name)
    }

    pub fn is_internal_decorator(&self, name: &str) -> bool {
        self.internal_decorators.contains(&name)
    }

    pub fn is_valid_for_backend(&self, name: &str, target: CompilationTarget) -> bool {
        if self.is_gpu_decorator(name) {
            return false;
        }
        if self.is_wasm_only(name) {
            return target == CompilationTarget::Wasm;
        }
        true
    }

    pub fn is_wasm_only(&self, name: &str) -> bool {
        self.wasm_only_decorators.contains(&name)
    }

    /// Returns all GPU decorator names. Used by `is_shader_file` to detect
    /// shader files from parsed AST without hardcoding decorator names.
    pub fn gpu_decorator_names(&self) -> &[&'static str] {
        &self.gpu_decorators
    }

    /// Should this decorator be skipped (not emitted) when generating
    /// attributes for the given backend?
    pub fn should_skip_for_backend(&self, name: &str, target: CompilationTarget) -> bool {
        if self.is_gpu_decorator(name) {
            return true;
        }
        if self.is_framework_decorator(name) {
            return true;
        }
        if self.is_internal_decorator(name) {
            return true;
        }
        if self.is_wasm_only(name) && target != CompilationTarget::Wasm {
            return true;
        }
        false
    }
}

impl Default for DecoratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_decorators_classified() {
        let reg = DecoratorRegistry::new();
        assert!(reg.is_gpu_decorator("vertex"));
        assert!(reg.is_gpu_decorator("fragment"));
        assert!(reg.is_gpu_decorator("compute"));
        assert!(!reg.is_gpu_decorator("test"));
    }

    #[test]
    fn test_framework_decorators_classified() {
        let reg = DecoratorRegistry::new();
        assert!(reg.is_framework_decorator("component"));
        assert!(reg.is_framework_decorator("game"));
        assert!(!reg.is_framework_decorator("test"));
    }

    #[test]
    fn test_should_skip_gpu_for_rust() {
        let reg = DecoratorRegistry::new();
        assert!(reg.should_skip_for_backend("vertex", CompilationTarget::Rust));
        assert!(reg.should_skip_for_backend("fragment", CompilationTarget::Rust));
        assert!(reg.should_skip_for_backend("compute", CompilationTarget::Rust));
    }

    #[test]
    fn test_should_not_skip_test_for_rust() {
        let reg = DecoratorRegistry::new();
        assert!(!reg.should_skip_for_backend("test", CompilationTarget::Rust));
    }

    #[test]
    fn test_export_only_for_wasm() {
        let reg = DecoratorRegistry::new();
        assert!(!reg.should_skip_for_backend("export", CompilationTarget::Wasm));
        assert!(reg.should_skip_for_backend("export", CompilationTarget::Rust));
    }

    #[test]
    fn test_profile_is_wrapping_decorator() {
        let reg = DecoratorRegistry::new();
        assert!(reg.is_wrapping_decorator("profile"));
    }
}
