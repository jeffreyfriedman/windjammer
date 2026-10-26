//! Method names that exist on some standard, runtime or common external type. A method call
//! whose name is in neither this list nor the program is reported (WJ0015). Generated from
//! stdlib_meta/, Rust std and the method names used across the repository corpora, plus every method declared in `std/**/*.wj`
//! (pinned by `std_methods_cover_the_embedded_std` below) and every method the interpreter implements
//! (`interpreter_methods_are_known`): the interpreter runs only programs `wj check` accepts, so a valid builtin
//! call must never be reported as an unknown method.

pub const KNOWN_METHODS: &[&str] = &[
    "about", "abs", "abs_diff", "acos", "add_days", "add_hours", "add_minutes",
    "add_result", "add_seconds", "all", "and", "and_modify", "and_then", "any", "append",
    "arg", "args", "as_bytes", "as_deref", "as_micros", "as_millis", "as_mut", "as_mut_ptr",
    "as_nanos", "as_ptr", "as_ref", "as_seconds", "as_secs", "as_secs_f32", "as_secs_f64",
    "as_slice", "as_str", "asin", "atan", "atan2", "author", "await", "back", "back_mut",
    "bad_request", "basic_auth", "bearer_auth", "binary", "binary_search",
    "binary_search_by", "binary_search_by_key", "bind", "body", "body_json", "body_string",
    "bool", "borrow", "borrow_mut", "bytes", "call_count", "capacity", "capitalize",
    "captures", "captures_all", "captures_iter", "case_insensitive", "cbrt", "ceil",
    "chain", "charCodeAt", "char_at", "char_count", "char_indices", "chars",
    "check_postconditions", "check_preconditions", "checked_add", "checked_div",
    "checked_mul", "checked_neg", "checked_pow", "checked_rem", "checked_sub", "choose",
    "choose_multiple", "chunk_by", "chunks", "chunks_exact", "clamp", "clear", "client_ip",
    "clone", "cloned", "close", "cmp", "collect", "concat", "connect", "contains",
    "contains_char", "contains_key", "context", "copied", "copysign", "cos", "count",
    "count_by", "count_ones", "count_zeros", "create_dir_all", "created", "cycle", "day",
    "decode_base64", "dedup", "dedup_by_key", "default_value", "delete", "delimiter",
    "difference", "div_euclid", "downcast_ref", "drain", "duration_since", "each_slice",
    "elapsed", "encode_base64", "end", "ends_with", "ensures", "entries", "entry",
    "enumerate", "eq", "eq_ignore_ascii_case", "err", "error", "execute", "exists", "exp",
    "expect", "expect_err", "extend", "extend_from_slice", "fetch_add", "fetch_all",
    "fetch_and", "fetch_max", "fetch_min", "fetch_one", "fetch_optional", "fetch_or",
    "fetch_sub", "fetch_update", "fetch_xor", "file_type", "fill", "filter", "filter_level",
    "filter_map", "filter_map_ok", "find", "find_all", "find_iter", "find_map", "first",
    "first_key_value", "first_mut", "flag", "flat_map", "flatten", "float", "floor", "fold",
    "for_each", "forbidden", "format", "format_standard", "fract", "frequencies",
    "fromCharCode", "from_bits", "from_char_code", "from_reader", "from_str",
    "from_str_radix", "from_utf8", "from_writer", "front", "front_mut", "full_match", "ge",
    "gen", "gen_bool", "gen_range", "get", "get_bool", "get_context", "get_header",
    "get_int", "get_int_at", "get_int_opt", "get_key_value", "get_many", "get_matches",
    "get_mut", "get_or_insert", "get_or_insert_with", "get_or_int", "get_param",
    "get_query_param", "get_return", "get_string", "get_string_at", "group_by", "groups",
    "gt", "hash", "head", "header", "headers", "help", "hour", "hours", "html", "hypot",
    "id", "index_of", "insert", "insert_str", "inspect", "internal_error", "intersection",
    "intersperse", "into", "into_boxed_slice", "into_bytes", "into_inner", "into_iter",
    "into_keys", "into_make_service", "into_owned", "into_string", "into_values",
    "is_alphabetic", "is_alphanumeric", "is_ascii", "is_ascii_alphabetic", "is_ascii_digit",
    "is_ascii_lowercase", "is_ascii_punctuation", "is_ascii_uppercase",
    "is_ascii_whitespace", "is_char_boundary", "is_digit", "is_dir", "is_disjoint",
    "is_empty", "is_eq", "is_err", "is_file", "is_finished", "is_finite", "is_ge", "is_gt",
    "is_infinite", "is_le", "is_leader", "is_lowercase", "is_lt", "is_match", "is_nan",
    "is_ne", "is_none", "is_none_or", "is_numeric", "is_ok", "is_ok_and", "is_present",
    "is_readonly", "is_ready", "is_sign_negative", "is_sign_positive", "is_some",
    "is_some_and", "is_sorted", "is_subset", "is_success", "is_superset", "is_uppercase",
    "is_whitespace", "is_zero", "iter", "iter_mut", "join", "json", "json_value", "key",
    "keys", "kill", "last", "last_index_of", "last_key_value", "last_mut", "le",
    "leading_zeros", "len", "len_utf8", "lines", "ln", "load", "location", "lock", "log",
    "log10", "log2", "lt", "make_ascii_lowercase", "make_ascii_uppercase", "map", "map_err",
    "map_or", "map_or_else", "map_while", "match", "match_indices", "matches", "max",
    "max_by", "max_by_key", "max_value", "metadata", "method", "migrate", "min", "min_by",
    "min_by_key", "min_value", "minute", "minutes", "month", "mul_add", "multiple", "name",
    "ne", "nest", "new", "new_server", "new_v4", "next", "next_back", "next_if",
    "no_content", "not_found", "notify_all", "notify_one", "now", "now_millis", "now_unix",
    "now_unix_nano", "nth", "num_milliseconds", "num_seconds", "ok", "ok_or", "ok_or_else",
    "only", "open", "or", "or_default", "or_else", "or_insert", "or_insert_with", "output",
    "overflowing_add", "padStart", "parse", "parse_iso8601", "partial_cmp", "partition",
    "partition_by", "patch", "path", "path_param", "payload_too_large", "peek", "peekable",
    "permissions", "pop", "pop_back", "pop_first", "pop_front", "pop_last", "position",
    "post", "pow", "powf", "powi", "pretty", "print", "println", "product", "push",
    "push_back", "push_front", "push_str", "put", "query", "query_all", "query_one",
    "query_param", "query_params", "range", "read", "read_dir", "read_line", "read_only",
    "read_to_string", "readonly", "receive", "receive_with_timeout", "record_call",
    "records", "recv", "recv_timeout", "reduce", "rem_euclid", "remove", "removeItem",
    "remove_entry", "remove_file", "repeat", "replace", "replace_all", "replacen",
    "required", "requires", "reserve", "resize", "response", "retain", "rev", "reverse",
    "rfind", "rotate_left", "rotate_right", "round", "route", "rows_affected", "rposition",
    "rsplit", "rsplit_once", "run", "sample_iter", "saturating_add", "saturating_mul",
    "saturating_sub", "scan", "search_params", "second", "send", "send_async", "send_text",
    "serve", "serve_static", "service_unavailable", "set", "setItem", "set_context",
    "set_header", "set_len", "set_return", "short", "shrink_to_fit", "shuffle", "signum",
    "sin", "size", "skip", "skip_while", "sleep_millis", "sleep_seconds", "slice", "sort",
    "sort_by", "sort_by_key", "sort_by_key_desc", "sort_unstable", "sort_unstable_by",
    "sorted", "sorted_by_key", "sorted_by_key_desc", "spawn", "split",
    "split_ascii_whitespace", "split_at", "split_first", "split_last", "split_off",
    "split_once", "split_terminator", "split_whitespace", "splitn", "sqrt", "stack_size",
    "start", "starts_with", "status", "status_code", "step_by", "store", "strip_prefix",
    "strip_suffix", "subsec_micros", "subsec_millis", "subsec_nanos", "substring",
    "success", "sum", "sum_by", "swap", "swap_remove", "symmetric_difference", "take",
    "take_while", "tally", "tan", "text", "text_response", "then", "then_some", "then_with",
    "thread", "timed_out", "timeout", "timeout_secs", "timestamp", "timestamp_millis",
    "toString", "to_ascii_lowercase", "to_ascii_uppercase", "to_be_bytes", "to_bits",
    "to_bytes", "to_digit", "to_float", "to_int", "to_le_bytes", "to_list", "to_lowercase",
    "to_owned", "to_rfc3339", "to_str", "to_string", "to_string_lossy", "to_uppercase",
    "to_vec", "too_many_requests", "total_cmp", "trailing_zeros", "trim", "trim_end",
    "trim_end_matches", "trim_matches", "trim_start", "trim_start_matches", "trunc",
    "truncate", "try_borrow", "try_borrow_mut", "try_into", "try_iter", "try_lock",
    "try_recv", "try_send", "unauthorized", "under", "union", "unique", "unique_by",
    "unpark", "unwrap", "unwrap_err", "unwrap_or", "unwrap_or_default", "unwrap_or_else",
    "unzip", "upgrade", "uri", "value_of", "values", "values_mut", "values_of", "var",
    "verify_called", "verify_called_times", "version", "wait", "wait_for_shutdown",
    "wait_timeout", "wait_while", "was_called", "windows", "with_args", "with_capacity",
    "with_error", "with_header", "with_state", "with_status", "wrapping_add",
    "wrapping_mul", "wrapping_sub", "write", "write_char", "write_record", "write_str",
    "xor", "year", "zip", "zip_with",
];

pub fn is_known(name: &str) -> bool {
    KNOWN_METHODS.binary_search(&name).is_ok()
        || name == crate::parser::optional_chain::OPT_CHAIN
        // ADR-0018 enumerable surface: one table (src/stdlib_surface)
        || crate::stdlib_surface::enumerable::is_surface_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_sorted_for_binary_search() {
        assert!(KNOWN_METHODS.windows(2).all(|w| w[0] < w[1]), "KNOWN_METHODS must be sorted and unique");
    }

    /// A valid call into the standard library must never be reported as an unknown method (the
    /// interpreter refuses programs `wj check` rejects, so a false WJ0015 would block a valid run).
    #[test]
    fn std_methods_cover_the_embedded_std() {
        fn walk(dir: &std::path::Path, missing: &mut Vec<String>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, missing);
                } else if p.extension().is_some_and(|x| x == "wj") {
                    for line in std::fs::read_to_string(&p).unwrap().lines() {
                        let t = line.trim_start();
                        if line.len() - t.len() < 2 {
                            continue;
                        }
                        let t = t.strip_prefix("pub ").unwrap_or(t);
                        let t = t.strip_prefix("async ").unwrap_or(t);
                        if let Some(rest) = t.strip_prefix("fn ") {
                            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                            if !name.is_empty() && !name.starts_with(char::is_uppercase) && !is_known(&name) {
                                missing.push(format!("{}: {name}", p.display()));
                            }
                        }
                    }
                }
            }
        }
        let mut missing = Vec::new();
        walk(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("std"), &mut missing);
        assert!(missing.is_empty(), "std methods missing from KNOWN_METHODS:\n{}", missing.join("\n"));
    }

    #[test]
    fn interpreter_methods_are_known() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interpreter/builtins");
        let mut missing = Vec::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(e.path()).unwrap();
            for line in text.lines() {
                let t = line.trim_start();
                let Some(rest) = ["r.pure(", "r.mutating(", "r.place("].iter().find_map(|p| t.strip_prefix(p)) else { continue };
                // `Kind::Option, "map", ...`
                if let Some(q) = rest.split('"').nth(1) {
                    if rest.starts_with("Kind::") && !is_known(q) {
                        missing.push(format!("{}: {q}", e.path().display()));
                    }
                }
            }
        }
        assert!(missing.is_empty(), "interpreter methods missing from KNOWN_METHODS:\n{}", missing.join("\n"));
    }
}
