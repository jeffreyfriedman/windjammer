//! Method names that exist on some standard, runtime or common external type. A method call
//! whose name is in neither this list nor the program is reported (WJ0015). Generated from
//! stdlib_meta/, Rust std and the method names used across the repository corpora; the test below keeps
//! it a superset of every method declared in `std/**/*.wj`.

pub const KNOWN_METHODS: &[&str] = &[
    "about", "abs", "abs_diff", "acos", "add_days", "add_hours", "add_minutes", "add_result",
    "add_seconds", "all", "and", "and_modify", "and_then", "any", "append", "arg", "args",
    "as_bytes", "as_deref", "as_millis", "as_mut", "as_mut_ptr", "as_ptr", "as_ref", "as_seconds",
    "as_secs", "as_secs_f64", "as_slice", "as_str", "asin", "atan", "atan2", "author", "await",
    "basic_auth", "bearer_auth", "begin", "binary_search", "binary_search_by",
    "binary_search_by_key", "bind", "body", "body_string", "bool", "borrow", "borrow_mut", "bytes",
    "call_count", "capacity", "capitalize", "captures", "captures_all", "captures_iter",
    "case_insensitive", "cbrt", "ceil", "chain", "charCodeAt", "char_at", "char_count",
    "char_indices", "chars", "check_postconditions", "check_preconditions", "checked_add",
    "checked_div", "checked_mul", "checked_neg", "checked_pow", "checked_rem", "checked_sub",
    "choose", "choose_multiple", "chunk_by", "chunks", "chunks_exact", "clamp", "clear",
    "client_ip", "clone", "cloned", "close", "cmp", "collect", "commit", "concat", "contains",
    "contains_char", "contains_key", "context", "copied", "cos", "count", "count_by", "count_ones",
    "count_zeros", "cycle", "day", "decode_base64", "dedup", "dedup_by_key", "default_value",
    "delete", "delimiter", "div_euclid", "drain", "duration_since", "each_slice", "elapsed",
    "encode_base64", "end", "ends_with", "ensures", "entries", "entry", "enumerate", "eq",
    "eq_ignore_ascii_case", "execute", "exp", "expect", "expect_err", "extend", "extend_from_slice",
    "fetch_all", "fetch_one", "fetch_optional", "file_type", "fill", "filter", "filter_level",
    "filter_map", "filter_map_ok", "find", "find_all", "find_iter", "find_map", "first",
    "first_mut", "flag", "flat_map", "flatten", "float", "floor", "fold", "for_each", "format",
    "format_standard", "fract", "frequencies", "fromCharCode", "from_bits", "from_char_code",
    "from_reader", "from_str_radix", "from_utf8", "from_writer", "full_match", "ge", "gen",
    "gen_bool", "gen_range", "get", "get_bool", "get_context", "get_header", "get_int",
    "get_int_at", "get_int_opt", "get_key_value", "get_many", "get_matches", "get_mut",
    "get_or_insert", "get_or_insert_with", "get_or_int", "get_param", "get_query_param",
    "get_return", "get_string", "get_string_at", "group_by", "groups", "gt", "header", "headers",
    "help", "hour", "hours", "hypot", "index_of", "insert", "inspect", "intersection",
    "intersperse", "into_inner", "into_iter", "into_keys", "into_make_service", "into_owned",
    "into_values", "is_alphabetic", "is_alphanumeric", "is_ascii", "is_ascii_alphabetic",
    "is_ascii_digit", "is_ascii_lowercase", "is_ascii_punctuation", "is_ascii_uppercase",
    "is_ascii_whitespace", "is_char_boundary", "is_digit", "is_dir", "is_empty", "is_err",
    "is_file", "is_finite", "is_infinite", "is_lowercase", "is_match", "is_nan", "is_none",
    "is_none_or", "is_numeric", "is_ok", "is_present", "is_readonly", "is_ready",
    "is_sign_negative", "is_sign_positive", "is_some", "is_some_and", "is_success", "is_uppercase",
    "is_whitespace", "iter", "iter_mut", "join", "keys", "kill", "last", "last_index_of",
    "last_mut", "le", "leading_zeros", "len", "lines", "ln", "location", "lock", "log", "log10",
    "log2", "lt", "make_ascii_lowercase", "make_ascii_uppercase", "map", "map_err", "map_or",
    "map_or_else", "map_while", "match", "match_indices", "max", "max_by", "max_by_key",
    "max_value", "metadata", "method", "migrate", "min", "min_by", "min_by_key", "min_value",
    "minute", "minutes", "month", "mul_add", "multiple", "name", "ne", "nest", "new", "new_server",
    "new_v4", "next", "now", "now_unix", "now_unix_nano", "nth", "num_milliseconds", "num_seconds",
    "ok", "ok_or", "ok_or_else", "open", "or", "or_default", "or_else", "or_insert",
    "or_insert_with", "output", "overflowing_add", "padStart", "parse", "parse_iso8601",
    "partial_cmp", "partition", "partition_by", "patch", "path", "path_param", "peekable",
    "permissions", "pop", "pop_back", "pop_front", "position", "post", "pow", "powf", "powi",
    "pretty", "print", "println", "product", "push", "push_back", "push_front", "push_str", "put",
    "query", "query_all", "query_one", "query_param", "query_params", "range", "read", "readonly",
    "receive", "receive_with_timeout", "record_call", "records", "recv", "rem_euclid", "remove",
    "removeItem", "repeat", "replace", "replace_all", "replacen", "required", "requires", "reserve",
    "resize", "response", "retain", "rev", "reverse", "rfind", "rollback", "rotate_left",
    "rotate_right", "round", "route", "rows_affected", "rposition", "rsplit", "sample_iter",
    "saturating_add", "saturating_mul", "saturating_sub", "scan", "search_params", "second", "send",
    "send_async", "send_text", "serve", "serve_static", "setItem", "set_context", "set_header",
    "set_len", "set_local", "set_return", "set_session", "short", "shrink_to_fit", "signum", "sin",
    "size", "skip", "skip_while", "sleep_millis", "sleep_seconds", "slice", "sort", "sort_by",
    "sort_by_key", "sort_by_key_desc", "sort_unstable", "sort_unstable_by", "sorted",
    "sorted_by_key", "sorted_by_key_desc", "spawn", "split", "split_at", "split_off", "split_once",
    "split_whitespace", "splitn", "sqrt", "start", "starts_with", "status", "status_code",
    "step_by", "strip_prefix", "strip_suffix", "substring", "success", "sum", "sum_by", "swap",
    "swap_remove", "take", "take_while", "tally", "tan", "text", "text_response", "then",
    "then_some", "timeout", "timeout_secs", "timestamp", "timestamp_millis", "toString",
    "to_ascii_lowercase", "to_ascii_uppercase", "to_bits", "to_bytes", "to_digit", "to_float",
    "to_int", "to_list", "to_lowercase", "to_owned", "to_rfc3339", "to_str", "to_string",
    "to_string_lossy", "to_uppercase", "to_vec", "trailing_zeros", "transaction", "trim",
    "trim_end", "trim_end_matches", "trim_matches", "trim_start", "trim_start_matches", "trunc",
    "truncate", "try_recv", "union", "unique", "unique_by", "unwrap", "unwrap_err", "unwrap_or",
    "unwrap_or_default", "unwrap_or_else", "unzip", "upgrade", "uri", "value_of", "values",
    "values_mut", "values_of", "var", "verify_called", "verify_called_times", "version", "wait",
    "wait_for_shutdown", "was_called", "windows", "with_args", "with_capacity", "with_error",
    "with_header", "with_state", "wrapping_add", "wrapping_mul", "wrapping_sub", "write",
    "write_record", "year", "zip", "zip_with",
];

pub fn is_known(name: &str) -> bool {
    KNOWN_METHODS.binary_search(&name).is_ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn list_is_sorted_and_covers_every_std_method() {
        assert!(super::KNOWN_METHODS.windows(2).all(|w| w[0] < w[1]), "KNOWN_METHODS must stay sorted and unique");
        let std_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("std");
        let mut missing = Vec::new();
        let mut stack = vec![std_dir];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "wj") {
                    for line in std::fs::read_to_string(&p).unwrap().lines() {
                        let t = line.trim_start().trim_start_matches("pub ").trim_start_matches("async ");
                        let Some(rest) = t.strip_prefix("fn ") else { continue };
                        let Some((name, params)) = rest.split_once('(') else { continue };
                        let p0 = params.trim_start().trim_start_matches('&').trim_start().trim_start_matches("mut ").trim_start();
                        if p0.starts_with("self") && !super::is_known(name.trim()) {
                            missing.push(name.trim().to_string());
                        }
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "std methods missing from KNOWN_METHODS (wj check would flag them WJ0015): {missing:?}");
    }
}
