//! Fix-oriented wording for errors the interpreter reports at run time (no rustc involved).
//!
//! The interpreter raises plain messages; this layer gives the common ones a stable code and a
//! one-line "what to write" help, so `wj run` output is as actionable as `wj check` output.

/// A classified runtime message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classified {
    pub code: &'static str,
    pub message: String,
    pub help: String,
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let rest = &text[s..];
    Some(&rest[..rest.find(end)?])
}

fn is_fallible(ty: &str) -> bool {
    matches!(ty, "Result" | "Option")
}

/// Classify one interpreter message; `None` when it already says what to do (or is not understood).
pub fn classify(msg: &str) -> Option<Classified> {
    let c = |code, message: String, help: &str| Some(Classified { code, message, help: help.to_string() });
    if let Some(ty) = between(msg, "`", "` doesn't implement `std::fmt::Display`") {
        let ty = ty.trim_start_matches('`');
        return match ty {
            t if is_fallible(t) => c(
                "WJ0032",
                format!("a `{t}` has no text form, so it cannot be printed with `${{..}}`"),
                "unwrap it first: `x ?? default`, `x?` inside a function that returns a `Result`, or `match x { .. }`",
            ),
            "Vec" | "HashMap" | "HashSet" => c(
                "WJ0032",
                format!("a `{ty}` has no text form, so it cannot be printed with `${{..}}`"),
                "print the elements with a `for` loop, or join them: `items.join(\", \")`",
            ),
            t => c(
                "WJ0032",
                format!("`{t}` has no text form, so it cannot be printed with `${{..}}`"),
                "print its fields (`${p.name}`), or implement `Display` for the type",
            ),
        };
    }
    if let (Some(m), Some(ty)) = (between(msg, "no method named `", "`"), between(msg, "value of type `", "`")) {
        return if is_fallible(ty) {
            c(
                "WJ0015",
                format!("a `{ty}` has no method `{m}`: the value may be an error or absent"),
                &format!("get the inner value first: `x ?? default`, or `x?`, then call `.{m}()` on it"),
            )
        } else if ty == "fn" {
            c(
                "WJ0015",
                format!("`{m}` was looked up on a function, not on a value"),
                "call the function first: write `name().{m}`, or check that the name refers to a value",
            )
        } else {
            c(
                "WJ0015",
                format!("`{ty}` has no method `{m}`"),
                "check the method name and the receiver's type; `wj check` reports the same error with a suggestion",
            )
        };
    }
    if let Some(rest) = msg.strip_prefix("indexing ") {
        if rest.starts_with("Iterator") {
            return c(
                "WJ0035",
                "an iterator cannot be indexed".to_string(),
                "collect it into a list first (`let items = it.collect()`), then index `items[i]`; or use `.nth(i)`",
            );
        }
    }
    if let Some(path) = between(msg, "not interpreted yet: `", "`") {
        let last = path.rsplit("::").next().unwrap_or(path);
        if path.starts_with("std::io::") && matches!(last, "read_all" | "read_stdin" | "read_all_stdin" | "read_line" | "stdin" | "read_input") {
            return c(
                "WJ0036",
                format!("`{path}` does not exist in Windjammer's standard library"),
                "read standard input with `use std::fs` and `fs::read_to_string(\"/dev/stdin\")`, then loop over `text.lines()`",
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_of_result() {
        let c = classify("`Result` doesn't implement `std::fmt::Display`").unwrap();
        assert_eq!(c.code, "WJ0032");
        assert!(c.help.contains("?? default"));
    }

    #[test]
    fn method_on_result() {
        let c = classify("no method named `len` found for value of type `Result`").unwrap();
        assert_eq!(c.code, "WJ0015");
        assert!(c.help.contains("?? default"));
    }

    #[test]
    fn iterator_index_and_stdin() {
        assert_eq!(classify("indexing Iterator with i64").unwrap().code, "WJ0035");
        let c = classify("not interpreted yet: `std::io::read_all` is not implemented by the interpreter [std.module_fn]").unwrap();
        assert!(c.help.contains("/dev/stdin"));
        assert!(classify("division by zero").is_none());
    }
}
