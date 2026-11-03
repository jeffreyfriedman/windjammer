//! Rewriting rustc messages into Windjammer terms: stable code, message and help text.
//! Anything not recognised falls back to `WJ0099` (an internal coverage gap), never raw rustc.

/// Result of translating one rustc error.
pub struct Translated {
    pub code: &'static str,
    pub message: String,
    pub help: Option<String>,
    /// A mechanical edit, when the hint is one (located in the `.wj` text by `rustc_map`).
    pub fix: Option<super::hints::FixHint>,
}

/// Replace Rust type spellings inside backticks with Windjammer ones (`i32` -> `int`).
pub fn wjify(msg: &str) -> String {
    msg.split('`')
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 1 {
                let mapped = crate::type_classification::rust_type_to_windjammer(part);
                let mapped = match mapped {
                    "&mut str" => "string",
                    m => m,
                };
                mapped.replace("&str", "string").replace("String", "string")
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("`")
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let rest = &text[s..];
    Some(&rest[..rest.find(end)?])
}

fn ownership_word(t: &str) -> &'static str {
    if t.starts_with('&') { "a borrowed" } else { "an owned" }
}

/// `(lhs, rhs)` of operator messages: "cannot add `rhs` to `lhs`", "cannot subtract `rhs` from `lhs`",
/// "cannot multiply `lhs` by `rhs`", "can't compare `lhs` with `rhs`" (also the `-assign` forms).
fn operator_operands(msg: &str) -> Option<(String, String)> {
    let first = between(msg, "`", "`")?;
    let rest = &msg[msg.find('`')? + first.len() + 2..];
    let second = between(rest, "`", "`")?;
    let (a, b) = (first.to_string(), second.to_string());
    if msg.starts_with("cannot add") || msg.starts_with("cannot subtract") {
        Some((b, a))
    } else if msg.starts_with("cannot multiply") || msg.starts_with("cannot divide") || msg.starts_with("can't compare") || msg.starts_with("cannot mod") {
        Some((a, b))
    } else {
        None
    }
}

/// `rust_code` is the rustc error code if any; `label` the primary span label.
pub fn translate(rust_code: Option<&str>, msg: &str, label: Option<&str>) -> Translated {
    let t = |code, message: String, help: Option<&str>| Translated {
        code,
        message,
        help: help.map(str::to_string),
        fix: None,
    };
    let name = between(msg, "`", "`").unwrap_or("value");
    match rust_code {
        Some("E0425") if msg.contains("function") => t(
            "WJ0002",
            format!("cannot find function `{name}` in this scope"),
            Some("define it, or import it with `use`"),
        ),
        Some("E0425") => t(
            "WJ0001",
            format!("cannot find value `{name}` in this scope"),
            Some("declare it before use, or check the spelling"),
        ),
        Some("E0412") | Some("E0422") | Some("E0433") if msg.contains("type") || msg.contains("struct") => t(
            "WJ0005",
            format!("cannot find type `{name}` in this scope"),
            Some("define it, or import it with `use`"),
        ),
        Some("E0432") | Some("E0433") => t(
            "WJ0006",
            format!("unresolved import or path `{name}`"),
            Some("check the module path"),
        ),
        Some("E0308") => {
            let ctx = label.unwrap_or(msg);
            let generic = "change the value or the declared type, or convert explicitly (`as`, `.to_string()`, `.parse()`)";
            match (between(ctx, "expected `", "`"), between(ctx, "found `", "`")) {
                (Some(e), Some(f)) => {
                    let (we, wf) = (wjify(&format!("`{e}`")), wjify(&format!("`{f}`")));
                    let (we, wf) = (we.trim_matches('`').to_string(), wf.trim_matches('`').to_string());
                    let message = if we == wf && e != f {
                        // two Rust spellings with one Windjammer name (`i32` / `i64` are both `int`): keep the raw pair
                        format!("mismatched types: expected `{e}`, found `{f}` (both are `{we}` in Windjammer: the integer widths differ)")
                    } else if we == wf {
                        format!("mismatched types: expected {} `{we}`, found {} `{wf}`", ownership_word(e), ownership_word(f))
                    } else {
                        format!("mismatched types: expected `{we}`, found `{wf}`")
                    };
                    match super::hints::mismatch(e, f) {
                        Some(h) => Translated { code: h.code, message, help: Some(h.help), fix: h.fix },
                        None => t("WJ0003", message, Some(generic)),
                    }
                }
                _ => t("WJ0003", "mismatched types".to_string(), Some(generic)),
            }
        }
        Some("E0423") => t(
            "WJ0002",
            format!("`{name}` is a type or module, not a function"),
            Some("construct a struct with `Name { field: value }`, or call its constructor (`Name::new(..)`)"),
        ),
        Some("E0608") => t(
            "WJ0035",
            wjify(&format!("this value cannot be indexed: {msg}")),
            Some("only lists, maps and strings can be indexed with `[..]`: for other values use a method or field, and collect an iterator into a list first"),
        ),
        Some("E0384") => t(
            "WJ0004",
            format!("cannot assign twice to immutable variable `{name}`"),
            Some("declare it with `let mut`"),
        ),
        Some("E0596") => t(
            "WJ0008",
            format!("cannot borrow `{name}` as mutable, as it is not declared `mut`"),
            Some("declare it with `let mut`"),
        ),
        Some("E0382") => t(
            "WJ0007",
            format!("value `{name}` is used after it was moved"),
            Some("use the new owner, use the value before it is passed on, or pass a copy with `.clone()`"),
        ),
        Some("E0507") => t(
            "WJ0016",
            format!("cannot move `{name}` out of a borrow (ownership: it is only borrowed here)"),
            Some("copy it with `.clone()`, or take the value by ownership"),
        ),
        Some("E0499") | Some("E0502") | Some("E0505") | Some("E0506") => t(
            "WJ0016",
            format!("`{name}` is borrowed in conflicting ways at the same time"),
            Some("finish using the first reference before taking the second"),
        ),
        Some("E0599") => {
            let ty = between(msg, "for ", " in").unwrap_or("this type");
            let ty = ty.trim_start_matches("struct ").trim_start_matches("enum ").trim_matches('`');
            t(
                "WJ0015",
                format!("no method `{name}` found for `{}`", wjify(&format!("`{ty}`")).trim_matches('`')),
                Some("check the method name, or implement it in an `impl` block"),
            )
        }
        Some("E0609") | Some("E0560") => t(
            "WJ0014",
            format!("no field `{name}` on this type"),
            Some("use one of the declared fields"),
        ),
        Some("E0063") => t(
            "WJ0009",
            format!("missing field `{name}` in struct initializer"),
            Some("add the missing field"),
        ),
        Some("E0004") => t(
            "WJ0010",
            format!("non-exhaustive match: {} not covered", between(msg, "patterns: ", " not covered").unwrap_or("some cases")),
            Some("add the missing arms or a final `_` arm"),
        ),
        Some("E0277") if msg.contains("cannot be built from an iterator") => match super::hints::collect_elements(msg) {
            Some(h) => Translated {
                code: h.code,
                message: super::hints::collect_message(msg).unwrap_or_else(|| wjify(msg)),
                help: Some(h.help),
                fix: h.fix,
            },
            None => t("WJ0029", wjify(msg), Some("convert each element with `.map(..)` before `.collect()`")),
        },
        Some("E0277") if operator_operands(msg).is_some() => {
            let (lhs, rhs) = operator_operands(msg).unwrap();
            let help = super::hints::mismatch(&lhs, &rhs)
                .map(|h| h.help)
                .unwrap_or_else(|| "convert one operand so both sides have the same type".to_string());
            t(
                "WJ0020",
                wjify(&format!("operator cannot combine `{lhs}` and `{rhs}`")),
                Some(&help),
            )
        }
        Some("E0277") if msg.contains("doesn't implement") || msg.contains("`Display`") || msg.contains("`Debug`") => t(
            "WJ0032",
            wjify(&format!("this type has no text form: {msg}")),
            Some("print its fields instead, or implement `Display` for the type"),
        ),
        Some("E0277") => t(
            "WJ0017",
            wjify(&format!("trait bound not satisfied: {}", msg.replace("the trait bound ", "").replace("is not satisfied", ""))).trim().to_string(),
            Some("implement the trait for the type, or use a type that has it"),
        ),
        Some("E0614") => t(
            "WJ0031",
            format!("`{}` is not a reference, so it cannot be dereferenced", wjify(&format!("`{name}`")).trim_matches('`')),
            Some("remove the `*`: Windjammer dereferences automatically, so write the value directly"),
        ),
        Some("E0782") => t(
            "WJ0034",
            "a trait cannot be used as a plain type".to_string(),
            Some("make the function generic over the trait (`fn f<T: Shape>(x: T)`), or store a `Box<dyn Shape>`"),
        ),
        Some("E0606") | Some("E0604") => t(
            "WJ0020",
            wjify(msg),
            Some("this cast is not allowed: convert in two steps (for example `x as int as float`), or parse/format the value instead"),
        ),
        Some("E0369") | Some("E0368") | Some("E0600") => t(
            "WJ0020",
            wjify(msg),
            Some("this operator does not apply to that type: convert the operands to a common numeric type first"),
        ),
        Some("E0046") | Some("E0407") => t(
            "WJ0018",
            wjify(msg),
            Some("define the missing methods in the `impl` block"),
        ),
        Some("E0603") | Some("E0616") => t(
            "WJ0019",
            wjify(msg),
            Some("declare the item `pub`"),
        ),
        Some("E0061") => t(
            "WJ0013",
            wjify(msg),
            Some("pass exactly the declared number of arguments"),
        ),
        Some("E0428") | Some("E0201") | Some("E0124") => t(
            "WJ0021",
            wjify(msg),
            Some("rename one of them or delete the duplicate"),
        ),
        Some("E0106") | Some("E0597") | Some("E0515") | Some("E0716") | Some("E0373") => t(
            "WJ0022",
            wjify(msg),
            Some("return an owned value instead of a reference to a local"),
        ),
        _ if msg.contains("missing boundary signature") => t(
            "WJ0098",
            "the compiler could not find a callee's signature".to_string(),
            Some("rebuild dependencies so their metadata is regenerated"),
        ),
        _ => t(
            "WJ0099",
            format!("the generated code was rejected: {}", wjify(msg)),
            Some("this is a compiler coverage gap; please report the program"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_translate_and_unknown_become_internal() {
        let t = translate(Some("E0382"), "borrow of moved value: `items`", None);
        assert_eq!((t.code, t.message.as_str()), ("WJ0007", "value `items` is used after it was moved"));
        let t = translate(Some("E0499"), "cannot borrow `v` as mutable more than once at a time", None);
        assert_eq!(t.code, "WJ0016");
        let t = translate(Some("E9999"), "something odd with `i32`", None);
        assert_eq!(t.code, "WJ0099");
        assert!(t.message.contains("`int`"));
    }

    #[test]
    fn type_spellings_are_translated() {
        let t = translate(Some("E0308"), "mismatched types", Some("expected `i32`, found `&str`"));
        assert_eq!(t.message, "mismatched types: expected `int`, found `string`");
    }

    #[test]
    fn function_vs_value_not_found() {
        assert_eq!(translate(Some("E0425"), "cannot find function `f` in this scope", None).code, "WJ0002");
        assert_eq!(translate(Some("E0425"), "cannot find value `x` in this scope", None).code, "WJ0001");
    }
}
