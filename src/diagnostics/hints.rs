//! "What to write instead": fix-oriented hints for the common type errors.
//!
//! Pure functions from the type spellings rustc reports (`String`, `&str`, `usize`, `Option<i64>`)
//! to a one-line Windjammer fix suggestion plus, where the change is mechanical, a [`FixHint`] that
//! `rustc_map` turns into a real edit once it has located the expression in the `.wj` source.

/// How a mechanical fix edits the expression rustc complained about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixAnchor {
    /// Wrap the whole expression: insert `prefix` before it and `suffix` after it.
    Expr { prefix: String, suffix: String },
    /// Insert `insert` right before the first (unique) occurrence of `needle` on the line
    /// (`.cloned()` before `.collect`).
    BeforeText { needle: String, insert: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixHint {
    pub description: String,
    pub anchor: FixAnchor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// Registry code to report (`WJ0003` for plain mismatches, `WJ0029` for collect element types).
    pub code: &'static str,
    pub help: String,
    pub fix: Option<FixHint>,
}

fn strip_lifetimes(t: &str) -> String {
    let t = t.trim();
    if let Some(rest) = t.strip_prefix("&'") {
        if let Some(i) = rest.find(' ') {
            return format!("&{}", strip_lifetimes(&rest[i + 1..]));
        }
    }
    t.to_string()
}

fn is_signed_or_float(t: &str) -> bool {
    matches!(t, "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "f32" | "f64")
}

fn is_int(t: &str) -> bool {
    matches!(t, "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize")
}

fn is_float(t: &str) -> bool {
    matches!(t, "f32" | "f64")
}

/// Windjammer name of a numeric Rust type as a cast target.
fn cast_name(t: &str) -> &str {
    match t {
        "i64" => "int",
        "usize" | "u64" => "uint",
        "f64" => "float",
        other => other,
    }
}

fn wrap(description: &str, prefix: &str, suffix: &str) -> Option<FixHint> {
    Some(FixHint {
        description: description.to_string(),
        anchor: FixAnchor::Expr { prefix: prefix.to_string(), suffix: suffix.to_string() },
    })
}

fn hint(help: String, fix: Option<FixHint>) -> Option<Hint> {
    Some(Hint { code: "WJ0003", help, fix })
}

/// Hint for `mismatched types: expected E, found F` (rustc spellings).
pub fn mismatch(expected: &str, found: &str) -> Option<Hint> {
    let (e, f) = (strip_lifetimes(expected), strip_lifetimes(found));
    let (e, f) = (e.as_str(), f.as_str());
    let owned = |t: &str| t == "String";
    let borrowed = |t: &str| matches!(t, "&str" | "&String" | "&&str" | "&mut str");
    if owned(e) && borrowed(f) {
        return hint(
            "add `.to_string()`: this place needs an owned `string`, the value is borrowed".into(),
            wrap("add `.to_string()`", "", ".to_string()"),
        );
    }
    if borrowed(e) && owned(f) {
        return hint(
            "pass the value as it is: Windjammer borrows for you. If this still errors, it is a compiler bug: please report it".into(),
            None,
        );
    }
    if let Some(inner) = e.strip_prefix("Option<").and_then(|x| x.strip_suffix('>')) {
        if inner == f {
            return hint(
                format!("wrap the value in `Some(..)`, or write `None`: this place needs an optional `{}`", wj(inner)),
                wrap("wrap the value in `Some(..)`", "Some(", ")"),
            );
        }
    }
    if let Some(inner) = f.strip_prefix("Option<").and_then(|x| x.strip_suffix('>')) {
        if inner == e {
            return hint(
                "the value may be absent: give it a default with `x ?? default`, propagate with `x?`, or handle it with `match`".into(),
                None,
            );
        }
    }
    if (is_int(e) && f == "usize") || (e == "usize" && is_int(f)) {
        let to = if e == "usize" { "uint" } else { "int" };
        return hint(
            "`len()`, `count()` and indices are `usize` when compiled but `int` is 64-bit: convert with `as int` (for example `let n: int = v.len() as int`), or compare `i < v.len() as int`; the other way round, write `as uint`".into(),
            wrap(&format!("convert with `as {to}`"), "", &format!(" as {to}")),
        );
    }
    if is_float(e) && is_int(f) {
        return hint(
            format!("integers are not converted to floats implicitly: write `x as {}`", cast_name(e)),
            wrap(&format!("convert with `as {}`", cast_name(e)), "", &format!(" as {}", cast_name(e))),
        );
    }
    if is_int(e) && is_float(f) {
        return hint(
            format!("floats are not converted to integers implicitly: write `x as {}` (this truncates)", cast_name(e)),
            wrap(&format!("convert with `as {}`", cast_name(e)), "", &format!(" as {}", cast_name(e))),
        );
    }
    if (is_int(e) && is_int(f) || is_float(e) && is_float(f)) && e != f {
        let n = cast_name(e);
        return hint(
            format!("`{}` and `{}` are different widths: convert with `as {n}`", wj(f), wj(e)),
            wrap(&format!("convert with `as {n}`"), "", &format!(" as {n}")),
        );
    }
    if matches!(e, "String" | "&str") && f == "char" || e == "char" && matches!(f, "String" | "&str") {
        return hint(
            "`char` and `string` are different types: use `c.to_string()` to make a string from a character, or `s.chars()` to walk a string".into(),
            if f == "char" { wrap("add `.to_string()`", "", ".to_string()") } else { None },
        );
    }
    if (is_signed_or_float(e) || e == "bool") && (f == "String" || f == "&str") {
        return hint(format!("a `string` is not a `{}`: parse it with `s.parse::<{}>()`", wj(e), wj(e)), None);
    }
    if (f == "i64" || f == "f64" || f == "bool" || f == "usize") && (e == "String" || e == "&str") {
        return hint("a number is not a `string`: use `x.to_string()` or `\"${x}\"`".into(), wrap("add `.to_string()`", "", ".to_string()"));
    }
    if e == "()" {
        return hint(
            format!("a block's value is its last expression: an `if` without `else` has no value, so add an `else` branch or end the branch with a statement (the `{}` here is dropped); or write `return value`", wj(f)),
            None,
        );
    }
    if f == "()" {
        return hint(
            "this block ends without a value: end it with the expression to return, with no trailing `;`".into(),
            None,
        );
    }
    None
}

fn wj(t: &str) -> String {
    crate::type_classification::rust_type_to_windjammer(t).to_string()
}

/// `Vec<String>` -> `Vec<string>` for messages.
fn wj_collection(t: &str) -> String {
    t.replace("String", "string")
}

/// Message for the collect element error: ``cannot collect `&str` items into `Vec<string>` ``.
pub fn collect_message(msg: &str) -> Option<String> {
    let built = between(msg, "a value of type `", "`")?;
    let from = strip_lifetimes(between(msg, "elements of type `", "`")?);
    Some(format!("cannot collect `{from}` items into `{}`", wj_collection(built)))
}

/// Hint for `a value of type V cannot be built from an iterator over elements of type X`.
pub fn collect_elements(msg: &str) -> Option<Hint> {
    let built = between(msg, "a value of type `", "`")?;
    let from = between(msg, "elements of type `", "`")?;
    let want = built.strip_prefix("Vec<").and_then(|x| x.strip_suffix('>')).unwrap_or(built);
    let got = strip_lifetimes(from);
    let (help, insert) = match (want, got.as_str()) {
        ("String", "&str") | ("String", "&&str") => (
            "the iterator yields borrowed text: add `.map(|s| s.to_string())` before `.collect()`".to_string(),
            ".map(|s| s.to_string())".to_string(),
        ),
        (w, g) if g.strip_prefix('&') == Some(w) => (
            format!("the elements are references (`&{}`): add `.cloned()` before `.collect()`", wj(w)),
            ".cloned()".to_string(),
        ),
        _ => (
            format!("the element type must match: build a `{}` from `{}` by converting each element with `.map(..)` before `.collect()`", wj(built), wj(&got)),
            String::new(),
        ),
    };
    let fix = (!insert.is_empty()).then(|| FixHint {
        description: format!("insert `{insert}` before `.collect()`"),
        anchor: FixAnchor::BeforeText { needle: ".collect".into(), insert },
    });
    Some(Hint { code: "WJ0029", help, fix })
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let rest = &text[s..];
    Some(&rest[..rest.find(end)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs() {
        assert!(mismatch("String", "&str").unwrap().help.contains(".to_string()"));
        assert!(mismatch("&str", "String").unwrap().help.contains("borrows for you"));
        assert!(mismatch("i64", "usize").unwrap().help.contains("as int"));
        assert!(mismatch("Option<i64>", "i64").unwrap().help.contains("Some("));
        assert!(mismatch("f32", "f64").unwrap().help.contains("as f32"));
        assert!(mismatch("Vec<i64>", "i64").is_none());
    }

    /// No hint may advise something the linters reject (`&x` at a call site is W0004, `.as_str()` is W0012).
    #[test]
    fn hints_never_advise_linted_constructs() {
        let types = ["String", "&str", "&String", "i64", "usize", "i32", "u32", "f32", "f64", "Option<i64>", "Option<String>", "()", "char", "bool"];
        for e in types {
            for f in types {
                let Some(h) = mismatch(e, f) else { continue };
                assert!(!h.help.contains("`&x`") && !h.help.contains(".as_str()"), "{e}/{f}: {}", h.help);
                if let Some(FixHint { anchor: FixAnchor::Expr { prefix, suffix }, .. }) = &h.fix {
                    assert!(!prefix.starts_with('&') && !suffix.contains("as_str"), "{e}/{f}");
                }
            }
        }
    }

    #[test]
    fn collect_cases() {
        let h = collect_elements("a value of type `Vec<String>` cannot be built from an iterator over elements of type `&String`").unwrap();
        assert_eq!(h.code, "WJ0029");
        assert!(h.help.contains(".cloned()"));
        let h = collect_elements("a value of type `Vec<String>` cannot be built from an iterator over elements of type `&str`").unwrap();
        assert!(h.help.contains("to_string"));
    }
}
