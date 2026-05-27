/// Delegates to the centralized `method_registry` module.
///
/// All functions here are thin wrappers preserving API compatibility
/// while the codebase migrates to direct `method_registry` calls.
pub fn method_mutates_receiver(method: &str) -> bool {
    crate::method_registry::mutates_receiver(method)
}

pub fn is_common_stdlib_method(method: &str) -> bool {
    crate::method_registry::is_common_stdlib_method(method)
}

pub fn method_returns_iterator(method: &str) -> bool {
    crate::method_registry::returns_iterator(method)
}

pub fn is_map_key_method(method: &str) -> bool {
    crate::method_registry::is_map_key_method(method)
}

pub fn is_index_taking_method(method: &str) -> bool {
    crate::method_registry::is_index_taking_method(method)
}

pub fn is_closure_taking_method(method: &str) -> bool {
    crate::method_registry::is_closure_taking_method(method)
}

/// Module names from `use std::…` that map to `windjammer_runtime::*` imports.
/// Calls must use `module::fn`, not `module.fn`.
/// Do not include common variable names (e.g. `dialog`, `log`, `io`).
pub fn is_runtime_std_module(name: &str) -> bool {
    matches!(
        name,
        "strings"
            | "json"
            | "time"
            | "math"
            | "random"
            | "http"
            | "mime"
            | "subprocess"
            | "async_runtime"
            | "async"
            | "cli"
            | "crypto"
            | "csv"
            | "db"
            | "regex"
            | "testing"
            | "game"
            | "env"
    )
}

/// Runtime std modules whose Rust implementations take `AsRef<str>` for Windjammer `string` params.
/// String literals must pass through as `&str` — no `.to_string()` / `&"-".to_string()`.
pub fn runtime_std_module_uses_asref_str(module: &str) -> bool {
    matches!(module, "strings" | "json" | "regex" | "csv" | "mime" | "http" | "env")
}
