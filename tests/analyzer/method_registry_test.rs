#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Tests for centralized method behavior registry.
///
/// The method registry replaces scattered string-matching heuristics
/// with a single, data-driven source of truth for stdlib method behaviors.
///
/// Philosophy: "No workarounds, no tech debt, only proper fixes."
/// - No string prefix/suffix matching for method behavior detection
/// - No game-specific hardcoded method names ("damage", "smooth_follow")
/// - One canonical registry, not 6 divergent copies
#[test]
fn test_known_mutating_methods_are_registered() {
    use windjammer::method_registry;

    let mutating = [
        "push",
        "pop",
        "insert",
        "remove",
        "clear",
        "append",
        "extend",
        "drain",
        "truncate",
        "resize",
        "retain",
        "sort",
        "sort_by",
        "sort_by_key",
        "sort_unstable",
        "sort_unstable_by",
        "dedup",
        "reverse",
        "swap",
        "swap_remove",
        "reserve",
        "shrink_to_fit",
        "split_off",
        "fill",
        "rotate_left",
        "rotate_right",
        "set_len",
        "push_str",
        "push_front",
        "push_back",
        "pop_front",
        "pop_back",
        "make_ascii_lowercase",
        "make_ascii_uppercase",
        "take",
        "replace",
        "get_or_insert",
        "get_or_insert_with",
        "entry",
    ];

    for method in &mutating {
        assert!(
            method_registry::mutates_receiver(method),
            "'{}' should be registered as mutating receiver",
            method
        );
    }
}

#[test]
fn test_known_readonly_methods_are_registered() {
    use windjammer::method_registry;

    let readonly = [
        "len",
        "is_empty",
        "contains",
        "contains_key",
        "get",
        "first",
        "last",
        "capacity",
        "keys",
        "values",
        "iter",
        "windows",
        "chunks",
        "enumerate",
        "clone",
        "to_string",
        "to_owned",
        "as_str",
        "as_ref",
        "as_slice",
        "as_bytes",
        "as_deref",
        "trim",
        "starts_with",
        "ends_with",
        "chars",
        "bytes",
        "split",
        "lines",
        "to_lowercase",
        "to_uppercase",
        "is_ascii",
        "abs",
        "ceil",
        "floor",
        "round",
        "sqrt",
        "powi",
        "powf",
        "sin",
        "cos",
        "tan",
        "log",
        "exp",
        "min",
        "max",
        "clamp",
        "display",
        "fmt",
        "cmp",
        "partial_cmp",
        "eq",
        "ne",
        "is_some",
        "is_none",
        "is_ok",
        "is_err",
        "unwrap",
        "unwrap_or",
        "unwrap_or_else",
        "unwrap_or_default",
        "expect",
        "map",
        "and_then",
        "or_else",
        "ok_or",
        "ok_or_else",
    ];

    for method in &readonly {
        assert!(
            !method_registry::mutates_receiver(method),
            "'{}' should NOT be registered as mutating receiver",
            method
        );
    }
}

#[test]
fn test_unknown_methods_default_to_non_mutating() {
    use windjammer::method_registry;

    // User-defined methods with "magic word" prefixes should NOT
    // be treated as mutating just because of their name
    let user_methods = [
        "set_theory",
        "insert_mode_check",
        "push_notification_handler",
        "clear_sky_render",
        "sort_algorithm_name",
        "update_description",
        "reset_reason_code",
        "damage",
        "smooth_follow",
        "look_at",
        "add_two_numbers",
        "remove_duplicates_algorithm",
    ];

    for method in &user_methods {
        assert!(
            !method_registry::mutates_receiver(method),
            "'{}' is a user-defined method - should NOT be treated as mutating \
             just because of string matching on its name prefix/suffix",
            method
        );
    }
}

#[test]
fn test_iterator_returning_methods() {
    use windjammer::method_registry;

    let iterator_methods = [
        "keys",
        "values",
        "iter",
        "iter_mut",
        "into_iter",
        "lines",
        "chars",
        "bytes",
        "split",
        "split_whitespace",
        "drain",
    ];

    for method in &iterator_methods {
        assert!(
            method_registry::returns_iterator(method),
            "'{}' should be registered as returning an iterator",
            method
        );
    }
}

#[test]
fn test_map_key_methods() {
    use windjammer::method_registry;

    let map_key_methods = ["contains_key", "get", "get_mut", "remove", "get_key_value"];

    for method in &map_key_methods {
        assert!(
            method_registry::is_map_key_method(method),
            "'{}' should be registered as a map key method",
            method
        );
    }
}

#[test]
fn test_index_taking_methods() {
    use windjammer::method_registry;

    let index_methods = [
        "remove",
        "swap_remove",
        "split_off",
        "swap",
        "drain",
        "insert",
    ];

    for method in &index_methods {
        assert!(
            method_registry::is_index_taking_method(method),
            "'{}' should be registered as an index-taking method",
            method
        );
    }
}

#[test]
fn test_closure_taking_methods() {
    use windjammer::method_registry;

    let closure_methods = [
        "filter",
        "any",
        "all",
        "find",
        "find_map",
        "position",
        "take_while",
        "skip_while",
        "map_while",
        "partition",
        "rposition",
    ];

    for method in &closure_methods {
        assert!(
            method_registry::is_closure_taking_method(method),
            "'{}' should be registered as a closure-taking method",
            method
        );
    }
}

#[test]
fn test_common_stdlib_method_names() {
    use windjammer::method_registry;

    let common = [
        "get",
        "get_mut",
        "remove",
        "contains_key",
        "contains",
        "insert",
        "push",
        "pop",
        "len",
        "is_empty",
        "iter",
        "keys",
        "values",
        "first",
        "last",
        "clear",
        "binary_search",
        "starts_with",
        "ends_with",
    ];

    for method in &common {
        assert!(
            method_registry::is_common_stdlib_method(method),
            "'{}' should be registered as a common stdlib method name",
            method
        );
    }
}

#[test]
fn test_no_prefix_matching_for_user_methods() {
    use windjammer::method_registry;

    // These method names happen to START WITH stdlib method names
    // but are user-defined and should NOT match
    let false_positives = [
        "push_notification",        // starts with "push" but is not Vec::push
        "insert_before_validation", // starts with "insert" but is not Vec::insert
        "remove_all_expired",       // starts with "remove" but may be &self
        "clear_cache_if_stale",     // starts with "clear" but may be &self
        "sort_algorithm",           // starts with "sort" but is not Vec::sort
        "extend_deadline",          // starts with "extend" but is not Vec::extend
    ];

    for method in &false_positives {
        assert!(
            !method_registry::mutates_receiver(method),
            "'{}' should NOT match as mutating - it's a user method, not a stdlib method. \
             The old string-prefix matching would have incorrectly flagged this!",
            method
        );
    }
}

#[test]
fn test_no_suffix_matching_for_user_methods() {
    use windjammer::method_registry;

    // Methods ending with "_mut" should NOT automatically be treated as mutating
    // unless they're actually registered stdlib methods
    let user_methods = [
        "get_current_state_mut",
        "find_player_mut",
        "calculate_result_mut",
    ];

    for method in &user_methods {
        // These are NOT stdlib methods - they should not be registered
        // The old `ends_with("_mut")` heuristic would have matched these
        assert!(
            !method_registry::is_known_stdlib_method(method),
            "'{}' should NOT be considered a known stdlib method",
            method
        );
    }
}

#[test]
fn test_get_mut_and_iter_mut_are_registered_correctly() {
    use windjammer::method_registry;

    // These ARE real stdlib methods that end with _mut
    assert!(method_registry::mutates_receiver("get_mut"));
    assert!(method_registry::mutates_receiver("iter_mut"));
    assert!(method_registry::mutates_receiver("values_mut"));

    // But they should be registered explicitly, not via suffix matching
    assert!(method_registry::is_known_stdlib_method("get_mut"));
    assert!(method_registry::is_known_stdlib_method("iter_mut"));
    assert!(method_registry::is_known_stdlib_method("values_mut"));
}
