//! Pure text helpers for format templates and interpolation specifiers.

/// Split an interpolation body `expr:spec` (as written inside `${...}`) into the
/// expression and an optional format specifier (`.2`, `?`, `>8`, `08.3`, `x`, ...).
///
/// Only a *top level* single `:` (not `::`, not nested inside brackets/strings)
/// whose right-hand side is a valid specifier counts; anything else is returned
/// unchanged as an expression.
pub fn split_interp_spec(body: &str) -> (&str, Option<&str>) {
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    let mut last_colon: Option<usize> = None;
    let mut in_str: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = in_str {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = Some(b'"'),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'|' if depth == 0 => {
                // closure parameter list `|x: int|` is not a specifier
                if let Some(end) = body[i + 1..].find('|') {
                    i += end + 2;
                    continue;
                }
            }
            b':' if depth == 0 => {
                if i + 1 < bytes.len() && bytes[i + 1] == b':' {
                    i += 2;
                    continue;
                }
                last_colon = Some(i);
            }
            _ => {}
        }
        i += 1;
    }
    if let Some(pos) = last_colon {
        let (expr, spec) = (&body[..pos], &body[pos + 1..]);
        if !expr.trim().is_empty() && is_format_spec(spec.trim()) {
            return (expr.trim_end(), Some(spec.trim()));
        }
    }
    (body, None)
}

/// `[[fill]align][sign][#][0][width][.precision][type]` with literal width/precision.
pub fn is_format_spec(spec: &str) -> bool {
    if spec.is_empty() {
        return false;
    }
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0;
    let is_align = |c: char| matches!(c, '<' | '^' | '>');
    if chars.len() >= 2 && is_align(chars[1]) {
        i = 2;
    } else if !chars.is_empty() && is_align(chars[0]) {
        i = 1;
    }
    if i < chars.len() && matches!(chars[i], '+' | '-') {
        i += 1;
    }
    if i < chars.len() && chars[i] == '#' {
        i += 1;
    }
    if i < chars.len() && chars[i] == '0' {
        i += 1;
    }
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    // type: ? x X o b e E, or `#?`
    if i < chars.len() {
        let rest: String = chars[i..].iter().collect();
        return matches!(rest.as_str(), "?" | "x" | "X" | "o" | "b" | "e" | "E" | "#?" | "x?" | "X?");
    }
    true
}

/// Convert a Rust-style template (`{}` / `{:?}` / `{{`) such as the one the parser
/// stores in `format` nodes into a Go `fmt` template (`%v` / `%+v` / `%.2f` ...).
/// Operates on the *already quoted* generated literal text, so quotes pass through.
pub fn go_printf_template(tmpl: &str) -> String {
    let chars: Vec<char> = tmpl.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                out.push(c);
                if i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 1;
                }
            }
            '%' => out.push_str("%%"),
            '{' if chars.get(i + 1) == Some(&'{') => {
                out.push('{');
                i += 1;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                out.push('}');
                i += 1;
            }
            '{' => {
                if let Some(rel) = chars[i..].iter().position(|&x| x == '}') {
                    let inner: String = chars[i + 1..i + rel].iter().collect();
                    out.push_str(&go_verb(inner.trim_start_matches(':')));
                    i += rel;
                } else {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

fn go_verb(spec: &str) -> String {
    if spec.is_empty() {
        return "%v".to_string();
    }
    if spec == "?" {
        return "%+v".to_string();
    }
    if spec == "#?" {
        return "%#v".to_string();
    }
    // align / width / precision / type
    let mut flags = String::new();
    let mut rest = spec;
    let first_two: Vec<char> = rest.chars().take(2).collect();
    if first_two.len() == 2 && matches!(first_two[1], '<' | '>' | '^') {
        if first_two[1] == '<' {
            flags.push('-');
        }
        rest = &rest[first_two[0].len_utf8() + 1..];
    } else if matches!(first_two.first(), Some('<' | '>' | '^')) {
        if first_two[0] == '<' {
            flags.push('-');
        }
        rest = &rest[1..];
    }
    if rest.starts_with('0') {
        flags.push('0');
        rest = &rest[1..];
    }
    let width: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    rest = &rest[width.len()..];
    let mut precision = String::new();
    if let Some(p) = rest.strip_prefix('.') {
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
        rest = &p[digits.len()..];
        precision = format!(".{digits}");
    }
    let verb = match rest {
        "" if !precision.is_empty() => "f",
        "" => "v",
        "?" => "+v",
        "x" => "x",
        "X" => "X",
        "o" => "o",
        "b" => "b",
        "e" => "e",
        "E" => "E",
        _ => "v",
    };
    format!("%{flags}{width}{precision}{verb}")
}

/// Result of converting a Rust format template to Windjammer interpolation.
pub type ConvertResult = Result<String, String>;

/// Convert the *inner source text* of a Rust format string literal (everything between
/// the quotes, escapes untouched) plus its argument source texts into the inner text of
/// an interpolated Windjammer string literal.
///
/// `positional` are the unnamed argument expressions, `named` the `name = expr` ones.
/// Fails (so callers leave the code untouched) for dynamic width/precision, unsupported
/// escapes, or argument count mismatches.
pub fn rust_template_to_interpolation(
    inner: &str,
    positional: &[String],
    named: &[(String, String)],
) -> ConvertResult {
    let chars: Vec<char> = inner.chars().collect();
    let mut out = String::new();
    let mut next_pos = 0usize;
    let mut used = vec![false; positional.len()];
    let mut used_named = vec![false; named.len()];
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                let Some(&n) = chars.get(i + 1) else {
                    return Err("dangling backslash".into());
                };
                if !matches!(n, 'n' | 't' | 'r' | '\\' | '"' | '\'') {
                    return Err(format!("unsupported escape `\\{n}` in format string"));
                }
                out.push('\\');
                out.push(n);
                i += 2;
            }
            '{' if chars.get(i + 1) == Some(&'{') => {
                out.push('{');
                i += 2;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                out.push('}');
                i += 2;
            }
            '}' => return Err("unmatched `}` in format string".into()),
            '{' => {
                let Some(rel) = chars[i..].iter().position(|&x| x == '}') else {
                    return Err("unterminated `{` in format string".into());
                };
                let body: String = chars[i + 1..i + rel].iter().collect();
                let (name_part, spec) = match body.find(':') {
                    Some(p) => (body[..p].trim().to_string(), Some(body[p + 1..].to_string())),
                    None => (body.trim().to_string(), None),
                };
                if let Some(s) = &spec {
                    if s.contains('$') || s.contains('*') {
                        return Err("dynamic width/precision is not auto-convertible".into());
                    }
                }
                let expr: String = if name_part.is_empty() {
                    let idx = next_pos;
                    next_pos += 1;
                    let Some(a) = positional.get(idx) else {
                        return Err("missing format argument".into());
                    };
                    used[idx] = true;
                    a.clone()
                } else if name_part.chars().all(|ch| ch.is_ascii_digit()) {
                    let idx: usize = name_part.parse().map_err(|_| "bad index".to_string())?;
                    let Some(a) = positional.get(idx) else {
                        return Err("format index out of range".into());
                    };
                    used[idx] = true;
                    a.clone()
                } else if let Some(pos) = named.iter().position(|(n, _)| *n == name_part) {
                    used_named[pos] = true;
                    named[pos].1.clone()
                } else {
                    name_part.clone()
                };
                let expr = expr.trim();
                if expr.is_empty() {
                    return Err("empty format argument".into());
                }
                out.push_str("${");
                out.push_str(expr);
                if let Some(s) = spec {
                    let s = s.trim();
                    if !s.is_empty() {
                        if !is_format_spec(s) {
                            return Err(format!("unsupported format spec `{s}`"));
                        }
                        out.push(':');
                        out.push_str(s);
                    }
                }
                out.push('}');
                i += rel + 1;
            }
            '$' => {
                // `$` directly before `{`/`$` in the output would start an interpolation.
                let next = chars.get(i + 1).copied();
                if matches!(next, Some('$') | Some('{')) {
                    out.push_str("$$");
                } else {
                    out.push('$');
                }
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    if used.iter().any(|u| !u) || used_named.iter().any(|u| !u) {
        return Err("unused format argument".into());
    }
    Ok(out)
}

/// Does this template inner text contain a `{..}` placeholder (not `{{`)?
pub fn template_has_placeholder(inner: &str) -> bool {
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '{' if chars.get(i + 1) == Some(&'{') => i += 2,
            '}' if chars.get(i + 1) == Some(&'}') => i += 2,
            '{' => return true,
            _ => i += 1,
        }
    }
    false
}

/// Undo `{{`/`}}` escapes (a template without placeholders becomes plain text).
pub fn unescape_braces(inner: &str) -> String {
    inner.replace("{{", "{").replace("}}", "}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn spec_split() {
        assert_eq!(split_interp_spec("x"), ("x", None));
        assert_eq!(split_interp_spec("x:.2"), ("x", Some(".2")));
        assert_eq!(split_interp_spec("p.x:>8.3"), ("p.x", Some(">8.3")));
        assert_eq!(split_interp_spec("a::b"), ("a::b", None));
        assert_eq!(split_interp_spec("f(a, b):?"), ("f(a, b)", Some("?")));
        assert_eq!(split_interp_spec("m[\"k:v\"]"), ("m[\"k:v\"]", None));
        assert_eq!(split_interp_spec("n:08.3"), ("n", Some("08.3")));
        assert_eq!(split_interp_spec("n:#?"), ("n", Some("#?")));
        assert_eq!(split_interp_spec("|x: int| x"), ("|x: int| x", None));
    }

    #[test]
    fn convert_basic() {
        assert_eq!(
            rust_template_to_interpolation("a {} b {:?}", &v(&["x", "y"]), &[]).unwrap(),
            "a ${x} b ${y:?}"
        );
        assert_eq!(
            rust_template_to_interpolation("{{}} {0} {0}", &v(&["x"]), &[]).unwrap(),
            "{} ${x} ${x}"
        );
        assert_eq!(
            rust_template_to_interpolation("{name}={v:.2}", &[], &[("v".into(), "a.b".into())])
                .unwrap(),
            "${name}=${a.b:.2}"
        );
        assert_eq!(
            rust_template_to_interpolation("cost $ {}", &v(&["c"]), &[]).unwrap(),
            "cost $ ${c}"
        );
        assert_eq!(
            rust_template_to_interpolation("${}", &v(&["c"]), &[]).unwrap(),
            "$$${c}"
        );
    }

    #[test]
    fn convert_errors() {
        assert!(rust_template_to_interpolation("{:w$}", &v(&["x"]), &[]).is_err());
        assert!(rust_template_to_interpolation("{}", &[], &[]).is_err());
        assert!(rust_template_to_interpolation("x", &v(&["y"]), &[]).is_err());
        assert!(rust_template_to_interpolation("\\u{41}", &[], &[]).is_err());
    }

    #[test]
    fn go_templates() {
        assert_eq!(go_printf_template("\"a {} {:?}\""), "\"a %v %+v\"");
        assert_eq!(go_printf_template("\"{:.2} {:>5} {:<5}\""), "\"%.2f %5v %-5v\"");
        assert_eq!(go_printf_template("\"100% {{x}}\""), "\"100%% {x}\"");
    }
}

/// Convert a Rust-style template (`inner` = text between the quotes, already lexed) plus the
/// generated JS argument expressions into the inside of a JS template literal.
pub fn js_template_from_format(inner: &str, args: &[String]) -> String {
    let chars: Vec<char> = inner.chars().collect();
    let mut out = String::new();
    let mut next = 0usize;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                out.push(c);
                if let Some(n) = chars.get(i + 1) {
                    out.push(*n);
                    i += 1;
                }
            }
            '{' if chars.get(i + 1) == Some(&'{') => {
                out.push('{');
                i += 1;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                out.push('}');
                i += 1;
            }
            '{' => {
                if let Some(rel) = chars[i..].iter().position(|&x| x == '}') {
                    let body: String = chars[i + 1..i + rel].iter().collect();
                    let spec = body.trim_start_matches(|c: char| c != ':').trim_start_matches(':');
                    let arg = if let Some(a) = args.get(next) {
                        a.clone()
                    } else {
                        "undefined".to_string()
                    };
                    next += 1;
                    let rendered = if spec == "?" || spec == "#?" {
                        format!("JSON.stringify({arg})")
                    } else if let Some(p) = spec.strip_prefix('.') {
                        let digits: String = p.chars().take_while(|d| d.is_ascii_digit()).collect();
                        format!("Number({arg}).toFixed({})", if digits.is_empty() { "0" } else { &digits })
                    } else {
                        arg
                    };
                    out.push_str("${");
                    out.push_str(&rendered);
                    out.push('}');
                    i += rel;
                } else {
                    out.push(c);
                }
            }
            '`' => out.push_str("\\`"),
            '$' if chars.get(i + 1) == Some(&'{') => out.push_str("\\$"),
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod js_tests {
    use super::*;

    #[test]
    fn js_templates() {
        assert_eq!(
            js_template_from_format("a {} {:.2} {:?}", &["x".into(), "y".into(), "z".into()]),
            "a ${x} ${Number(y).toFixed(2)} ${JSON.stringify(z)}"
        );
        assert_eq!(js_template_from_format("{{}}`", &[]), "{}\\`");
    }
}
