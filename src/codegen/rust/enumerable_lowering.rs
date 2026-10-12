//! Rust lowering of the enumerable collection surface (ADR-0018 feature 1).
//!
//! The surface table (`crate::stdlib_surface::enumerable`) names the runtime method; this module only renames the
//! call and inlines the std-only runtime source (`crates/windjammer-runtime/src/enumerable.rs`) as
//! `mod wj_enumerable`, so generated crates need no dependency. Ownership of the arguments is decided upstream
//! (the closure is an ordinary closure); nothing here inspects argument types.

use crate::codegen::rust::CodeGenerator;
use crate::stdlib_surface::enumerable;

const RUNTIME_SOURCE: &str = include_str!("../../../crates/windjammer-runtime/src/enumerable.rs");

/// `mod wj_enumerable { .. }`: the runtime source without its unit tests.
pub(super) fn inline_module() -> String {
    let src = RUNTIME_SOURCE
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(RUNTIME_SOURCE);
    format!(
        "#[allow(dead_code, clippy::all)]\nmod {} {{\n{}}}\n",
        enumerable::RUST_MODULE,
        src
    )
}

impl<'ast> CodeGenerator<'ast> {
    /// The Rust method name for `recv.method(args)`: the runtime name when `method` is a surface method the
    /// receiver's own type does not define.
    pub(in crate::codegen::rust) fn enumerable_rust_name(
        &mut self,
        receiver_type: Option<&str>,
        method: &str,
        argc: usize,
    ) -> Option<&'static str> {
        let m = enumerable::lookup(method, argc)?;
        if self.receiver_defines_method(receiver_type, method) {
            return None;
        }
        self.uses_enumerable = true;
        Some(m.rust)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn inlined_module_has_no_tests_and_both_traits() {
        let m = super::inline_module();
        assert!(m.starts_with("#[allow(dead_code, clippy::all)]\nmod wj_enumerable {"));
        assert!(m.contains("pub trait WjIterExt") && m.contains("pub trait WjCollExt"));
        assert!(!m.contains("mod tests"));
    }
}
