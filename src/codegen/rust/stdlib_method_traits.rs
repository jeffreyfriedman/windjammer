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
