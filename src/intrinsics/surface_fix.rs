//! Source-level surface-syntax migration (ADR-0014 Phase 1).
//!
//! One engine powers both `wj lint` / compile warnings (findings) and `wj fix`
//! (rewritten text). It is a small lexical scanner, not a regex: strings,
//! comments, char literals and `${..}` interpolations are understood, nested and
//! multi-line macro invocations are handled, and the rewrite is idempotent
//! (`fix(fix(x)) == fix(x)`).
//!
//! Rewrites:
//! * `println!("a {} b {:?}", x, y)` -> `println("a ${x} b ${y:?}")` (also print/panic/todo/unreachable/assert*)
//! * `eprintln!(..)` / `eprint!(..)` -> `stderr.println(..)` / `stderr.print(..)`
//! * `format!("v={}", x)` -> `"v=${x}"`
//! * `dbg!(x)` -> `debug(x)`
//! * `vec![a, b]` / `vec![x; n]` / `vec![]` -> `[a, b]` / `[x; n]` / `[]`
//!   (non-empty lists only where the list type is determined by the context:
//!   `let` initialisers, struct fields, `for .. in`; elsewhere left as-is)
//! * plain calls with legacy `{}` templates: `println("a {}", x)` -> `println("a ${x}")`

use super::template::{
    rust_template_to_interpolation, template_has_placeholder, unescape_braces,
};
use super::Intrinsic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Catalog code: W0007 (macro spelling), W0008 (`{}` format string).
    pub code: &'static str,
    pub offset: usize,
    pub end: usize,
    /// 1-based.
    pub line: usize,
    pub col: usize,
    pub original: String,
    /// Suggested replacement, `None` when it cannot be rewritten automatically.
    pub replacement: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct FixOutput {
    pub text: String,
    pub findings: Vec<Finding>,
}

impl FixOutput {
    pub fn changed_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.replacement.is_some())
            .count()
    }
    pub fn skipped_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.replacement.is_none())
            .count()
    }
}

/// Rewrite deprecated forms in `src`.
pub fn fix_source(src: &str) -> FixOutput {
    let mut rw = Rewriter::new(src);
    let text = rw.rewrite_range(src, 0, false);
    let mut findings = rw.findings;
    findings.sort_by_key(|f| (f.offset, f.code));
    findings.dedup_by(|a, b| a.offset == b.offset && a.code == b.code);
    FixOutput { text, findings }
}

/// Rewrite the Windjammer code fences (```windjammer / ```wj) of a Markdown document.
pub fn fix_markdown(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut lines = md.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        out.push_str(line);
        let t = line.trim();
        if t == "```windjammer" || t == "```wj" {
            let mut body = String::new();
            let mut closing = None;
            for l in lines.by_ref() {
                if l.trim() == "```" {
                    closing = Some(l);
                    break;
                }
                body.push_str(l);
            }
            out.push_str(&fix_source(&body).text);
            if let Some(c) = closing {
                out.push_str(c);
            }
        }
    }
    out
}

/// Findings only (for `wj lint` and compile-time warnings).
pub fn lint_source(src: &str) -> Vec<Finding> {
    fix_source(src).findings
}

struct Rewriter {
    line_starts: Vec<usize>,
    findings: Vec<Finding>,
}

const PLAIN_CALLS: [&str; 10] = [
    "print",
    "println",
    "eprint",
    "eprintln",
    "panic",
    "assert",
    "assert_eq",
    "assert_ne",
    "todo",
    "unreachable",
];

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}
fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Index just past a string literal starting at `i` (`text[i] == '"'`).
fn skip_string(text: &str, i: usize) -> usize {
    let b = text.as_bytes();
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return j + 1,
            b'$' if b.get(j + 1) == Some(&b'{') => {
                // interpolation: skip to the matching `}` (the WJ lexer ignores quotes here)
                let mut depth = 1;
                j += 2;
                while j < b.len() && depth > 0 {
                    match b[j] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
            }
            _ => j += 1,
        }
    }
    b.len()
}

/// Raw string `r"..."` / `r#"..."#` starting at `i` (`text[i] == 'r'`); returns end.
fn skip_raw_string(text: &str, i: usize) -> Option<usize> {
    let b = text.as_bytes();
    let mut j = i + 1;
    let mut hashes = 0;
    while b.get(j) == Some(&b'#') {
        hashes += 1;
        j += 1;
    }
    if b.get(j) != Some(&b'"') {
        return None;
    }
    j += 1;
    while j < b.len() {
        if b[j] == b'"' {
            let mut k = 0;
            while k < hashes && b.get(j + 1 + k) == Some(&b'#') {
                k += 1;
            }
            if k == hashes {
                return Some(j + 1 + hashes);
            }
        }
        j += 1;
    }
    Some(b.len())
}

/// Char literal at `i` (`text[i] == '\''`); `None` for lifetimes / labels.
fn skip_char_lit(text: &str, i: usize) -> Option<usize> {
    let rest = &text[i + 1..];
    let mut it = rest.char_indices();
    let (_, c) = it.next()?;
    if c == '\\' {
        let close = rest[1..].find('\'')?;
        return Some(i + 1 + 1 + close + 1);
    }
    let (n, c2) = it.next()?;
    if c2 == '\'' {
        return Some(i + 1 + n + 1);
    }
    None
}

fn skip_block_comment(text: &str, i: usize) -> usize {
    let b = text.as_bytes();
    let mut depth = 0;
    let mut j = i;
    while j + 1 < b.len() {
        if b[j] == b'/' && b[j + 1] == b'*' {
            depth += 1;
            j += 2;
        } else if b[j] == b'*' && b[j + 1] == b'/' {
            depth -= 1;
            j += 2;
            if depth == 0 {
                return j;
            }
        } else {
            j += 1;
        }
    }
    b.len()
}

fn skip_line_comment(text: &str, i: usize) -> usize {
    text[i..].find('\n').map(|p| i + p).unwrap_or(text.len())
}

/// Matching close delimiter for the opener at `open`.
fn find_matching(text: &str, open: usize) -> Option<usize> {
    let b = text.as_bytes();
    let mut stack: Vec<u8> = Vec::new();
    let mut j = open;
    while j < b.len() {
        match b[j] {
            b'(' | b'[' | b'{' => {
                stack.push(b[j]);
                j += 1;
            }
            b')' | b']' | b'}' => {
                let want = match b[j] {
                    b')' => b'(',
                    b']' => b'[',
                    _ => b'{',
                };
                if stack.pop() != Some(want) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(j);
                }
                j += 1;
            }
            b'"' => j = skip_string(text, j),
            b'/' if b.get(j + 1) == Some(&b'/') => j = skip_line_comment(text, j),
            b'/' if b.get(j + 1) == Some(&b'*') => j = skip_block_comment(text, j),
            b'\'' => j = skip_char_lit(text, j).unwrap_or(j + 1),
            b'r' if (j == 0 || !is_ident_char(b[j - 1]))
                && matches!(b.get(j + 1), Some(b'"') | Some(b'#')) =>
            {
                j = skip_raw_string(text, j).unwrap_or(j + 1)
            }
            _ => j += 1,
        }
    }
    None
}

/// Split `inner` at top-level `sep` bytes; returns byte ranges.
fn split_top_level(inner: &str, seps: &[u8]) -> Vec<(usize, usize)> {
    let b = inner.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    let mut j = 0;
    while j < b.len() {
        match b[j] {
            b'(' | b'[' | b'{' => {
                depth += 1;
                j += 1;
            }
            b')' | b']' | b'}' => {
                depth -= 1;
                j += 1;
            }
            b'"' => j = skip_string(inner, j),
            b'/' if b.get(j + 1) == Some(&b'/') => j = skip_line_comment(inner, j),
            b'/' if b.get(j + 1) == Some(&b'*') => j = skip_block_comment(inner, j),
            b'\'' => j = skip_char_lit(inner, j).unwrap_or(j + 1),
            c if depth == 0 && seps.contains(&c) => {
                out.push((start, j));
                j += 1;
                start = j;
            }
            _ => j += 1,
        }
    }
    out.push((start, b.len()));
    out
}

fn trim_range(text: &str, (s, e): (usize, usize)) -> (usize, usize) {
    let slice = &text[s..e];
    let lead = slice.len() - slice.trim_start().len();
    let trimmed = slice.trim();
    (s + lead, s + lead + trimmed.len())
}

/// `"..."` plain (non-raw) string literal -> inner text.
fn string_literal_inner(arg: &str) -> Option<&str> {
    let a = arg.trim();
    if a.len() >= 2 && a.starts_with('"') && a.ends_with('"') && skip_string(a, 0) == a.len() {
        Some(&a[1..a.len() - 1])
    } else {
        None
    }
}

/// `name = expr` (single `=`) -> (name, expr)
fn split_named_arg(arg: &str) -> Option<(String, String)> {
    let a = arg.trim();
    let b = a.as_bytes();
    if b.is_empty() || !is_ident_start(b[0]) {
        return None;
    }
    let mut j = 0;
    while j < b.len() && is_ident_char(b[j]) {
        j += 1;
    }
    let name = &a[..j];
    let rest = a[j..].trim_start();
    if rest.starts_with('=') && !rest.starts_with("==") && !rest.starts_with("=>") {
        Some((name.to_string(), rest[1..].trim().to_string()))
    } else {
        None
    }
}

impl Rewriter {
    fn new(src: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in src.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        Rewriter {
            line_starts,
            findings: Vec::new(),
        }
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(l) => l,
            Err(l) => l - 1,
        };
        (line + 1, offset - self.line_starts[line] + 1)
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &mut self,
        code: &'static str,
        offset: usize,
        original: &str,
        replacement: Option<String>,
        message: String,
    ) {
        let (line, col) = self.line_col(offset);
        self.findings.push(Finding {
            code,
            offset,
            end: offset + original.len(),
            line,
            col,
            original: original.to_string(),
            replacement,
            message,
        });
    }

    /// Rewrite one source range. `base` is the range's offset in the original file
    /// (only used for diagnostics positions). `ctx_list` marks the range as the
    /// element list of an enclosing rewritten `vec![..]`.
    fn rewrite_range(&mut self, text: &str, base: usize, ctx_list: bool) -> String {
        let b = text.as_bytes();
        let mut out = String::with_capacity(text.len());
        let mut i = 0usize;
        let mut depth = 0i32;
        while i < b.len() {
            let c = b[i];
            match c {
                b'/' if b.get(i + 1) == Some(&b'/') => {
                    let e = skip_line_comment(text, i);
                    out.push_str(&text[i..e]);
                    i = e;
                }
                b'/' if b.get(i + 1) == Some(&b'*') => {
                    let e = skip_block_comment(text, i);
                    out.push_str(&text[i..e]);
                    i = e;
                }
                b'"' => {
                    i = self.copy_string(text, i, base, &mut out);
                }
                b'\'' => {
                    if let Some(e) = skip_char_lit(text, i) {
                        out.push_str(&text[i..e]);
                        i = e;
                    } else {
                        out.push('\'');
                        i += 1;
                    }
                }
                b'(' | b'[' | b'{' => {
                    depth += 1;
                    out.push(c as char);
                    i += 1;
                }
                b')' | b']' | b'}' => {
                    depth -= 1;
                    out.push(c as char);
                    i += 1;
                }
                _ if is_ident_start(c) && (i == 0 || !is_ident_char(b[i - 1])) => {
                    let mut j = i;
                    while j < b.len() && is_ident_char(b[j]) {
                        j += 1;
                    }
                    let ident = &text[i..j];
                    // raw string prefix
                    if ident == "r" && matches!(b.get(j), Some(b'"') | Some(b'#')) {
                        if let Some(e) = skip_raw_string(text, i) {
                            out.push_str(&text[i..e]);
                            i = e;
                            continue;
                        }
                    }
                    let prev_sig = text[..i].trim_end().chars().last();
                    let after_dot = matches!(prev_sig, Some('.'))
                        || text[..i].ends_with("::")
                        || prev_word(text, i) == Some("fn");
                    // macro form: ident!(...)
                    if !after_dot
                        && b.get(j) == Some(&b'!')
                        && matches!(b.get(j + 1), Some(b'(') | Some(b'[') | Some(b'{'))
                    {
                        if let Some(intr) = Intrinsic::from_macro_name(ident) {
                            let direct = ctx_list
                                && depth == 0
                                && matches!(prev_sig, None | Some(',') | Some(';'));
                            let prefix_ok = direct || vec_context_ok(text, i);
                            if let Some((replacement, end)) =
                                self.try_macro(text, base, i, j, intr, prefix_ok, ctx_list)
                            {
                                out.push_str(&replacement);
                                i = end;
                                continue;
                            }
                        }
                    }
                    // plain call with a legacy `{}` template
                    if !after_dot && b.get(j) == Some(&b'(') && PLAIN_CALLS.contains(&ident) {
                        if let Some((replacement, end)) = self.try_plain_call(text, base, i, j) {
                            out.push_str(&replacement);
                            i = end;
                            continue;
                        }
                    }
                    out.push_str(ident);
                    i = j;
                }
                _ => {
                    let ch = text[i..].chars().next().unwrap();
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
        out
    }

    /// Copy a string literal, rewriting macros inside `${..}` interpolations.
    fn copy_string(&mut self, text: &str, i: usize, base: usize, out: &mut String) -> usize {
        let b = text.as_bytes();
        let end = skip_string(text, i);
        let mut j = i;
        let mut seg_start = i;
        while j < end {
            match b[j] {
                b'\\' => j += 2,
                b'$' if b.get(j + 1) == Some(&b'$') => j += 2,
                b'$' if b.get(j + 1) == Some(&b'{') => {
                    let body_start = j + 2;
                    let mut depth = 1;
                    let mut k = body_start;
                    while k < end && depth > 0 {
                        match b[k] {
                            b'{' => depth += 1,
                            b'}' => depth -= 1,
                            _ => {}
                        }
                        k += 1;
                    }
                    let body_end = if depth == 0 { k - 1 } else { k };
                    out.push_str(&text[seg_start..body_start]);
                    let rewritten =
                        self.rewrite_range(&text[body_start..body_end], base + body_start, false);
                    if rewritten.trim_start().starts_with('"')
                        && rewritten != &text[body_start..body_end]
                    {
                        out.push('(');
                        out.push_str(&rewritten);
                        out.push(')');
                    } else {
                        out.push_str(&rewritten);
                    }
                    seg_start = body_end;
                    j = body_end;
                }
                _ => j += 1,
            }
        }
        out.push_str(&text[seg_start..end.min(text.len())]);
        end
    }

    /// Returns (replacement text, end index in `text`) or `None` to leave untouched.
    #[allow(clippy::too_many_arguments)]
    fn try_macro(
        &mut self,
        text: &str,
        base: usize,
        start: usize,
        name_end: usize,
        intr: Intrinsic,
        prefix_ok: bool,
        _ctx_list: bool,
    ) -> Option<(String, usize)> {
        let name = &text[start..name_end];
        let open = name_end + 1;
        let close = find_matching(text, open)?;
        let inner = &text[open + 1..close];
        let original = &text[start..close + 1];
        let abs = base + start;
        let spans = split_top_level(inner, if intr == Intrinsic::Vec { b",;" } else { b"," });
        let has_semicolon = intr == Intrinsic::Vec && inner_has_top_level_semicolon(inner);

        // ---- vec![..] -> [..]
        if intr == Intrinsic::Vec {
            let msg = format!("`{name}!` is deprecated: use the list literal `[..]`");
            if inner.trim().is_empty() {
                self.record("W0007", abs, original, Some("[]".to_string()), msg);
                return Some(("[]".to_string(), close + 1));
            }
            if !prefix_ok {
                // The list type depends on the surrounding context; leave for manual migration.
                self.record(
                    "W0007",
                    abs,
                    original,
                    None,
                    format!("{msg} (not auto-fixable here: type-dependent position)"),
                );
                return None;
            }
            let _ = has_semicolon;
            let body = self.rewrite_range(inner, base + open + 1, true);
            let rep = format!("[{body}]");
            self.record("W0007", abs, original, Some(rep.clone()), msg);
            return Some((rep, close + 1));
        }

        // ---- everything else works on comma separated arguments
        let mut arg_ranges: Vec<(usize, usize)> =
            spans.iter().map(|r| trim_range(inner, *r)).collect();
        if arg_ranges.last().map(|r| r.0 == r.1).unwrap_or(false) {
            arg_ranges.pop(); // trailing comma
        }
        if arg_ranges.iter().any(|r| r.0 == r.1) {
            return None;
        }
        let args: Vec<String> = arg_ranges
            .iter()
            .map(|&(s, e)| self.rewrite_range(&inner[s..e], base + open + 1 + s, false))
            .collect();

        let skip = |this: &mut Self, why: &str| -> Option<(String, usize)> {
            this.record(
                "W0007",
                abs,
                original,
                None,
                format!("`{name}!` is deprecated: use `{}(..)` ({why})", intr.surface_spelling()),
            );
            None
        };

        let head = intr.surface_spelling();
        let rep = match intr {
            Intrinsic::Debug => {
                if args.len() != 1 {
                    return skip(self, "dbg! with several arguments");
                }
                format!("debug({})", args[0])
            }
            Intrinsic::Todo | Intrinsic::Unreachable if args.is_empty() => format!("{head}()"),
            _ => {
                let Some(msg_idx) = intr.message_arg_index() else {
                    return skip(self, "unsupported");
                };
                if args.len() <= msg_idx {
                    // no template: assert!(c), assert_eq!(a, b), println!()
                    if intr == Intrinsic::Format {
                        return skip(self, "format! without arguments");
                    }
                    format!("{head}({})", args.join(", "))
                } else {
                    let Some(tmpl_inner) = string_literal_inner(&args[msg_idx]) else {
                        return skip(self, "format template is not a string literal");
                    };
                    let rest = &args[msg_idx + 1..];
                    let body = match convert_template_args(tmpl_inner, rest) {
                        Ok(b) => b,
                        Err(e) => return skip(self, &e),
                    };
                    let lit = format!("\"{body}\"");
                    if intr == Intrinsic::Format {
                        lit
                    } else {
                        let mut parts: Vec<String> = args[..msg_idx].to_vec();
                        parts.push(lit);
                        format!("{head}({})", parts.join(", "))
                    }
                }
            }
        };
        let code = if arg_ranges.iter().skip(1).any(|_| true)
            && intr.message_arg_index().is_some_and(|m| {
                arg_ranges.len() > m + 1
                    || string_literal_inner(&args[m.min(args.len() - 1)])
                        .is_some_and(template_has_placeholder)
            }) {
            "W0008"
        } else {
            "W0007"
        };
        let target = if intr == Intrinsic::Format {
            "an interpolated string `\"..${x}..\"`".to_string()
        } else {
            format!("`{head}`")
        };
        let msg = if code == "W0008" {
            format!("`{name}!` with a `{{}}` format string is deprecated: use {target} with `${{..}}` interpolation")
        } else {
            format!("`{name}!` is deprecated: use {target}")
        };
        self.record(code, abs, original, Some(rep.clone()), msg);
        Some((rep, close + 1))
    }

    /// Legacy plain call with a `{}` template and trailing arguments.
    fn try_plain_call(
        &mut self,
        text: &str,
        base: usize,
        start: usize,
        name_end: usize,
    ) -> Option<(String, usize)> {
        let name = &text[start..name_end];
        let intr = Intrinsic::from_call_name(name)?;
        let msg_idx = intr.message_arg_index()?;
        let open = name_end;
        let close = find_matching(text, open)?;
        let inner = &text[open + 1..close];
        let spans = split_top_level(inner, b",");
        let mut ranges: Vec<(usize, usize)> = spans.iter().map(|r| trim_range(inner, *r)).collect();
        if ranges.last().map(|r| r.0 == r.1).unwrap_or(false) {
            ranges.pop();
        }
        if ranges.len() <= msg_idx + 1 || ranges.iter().any(|r| r.0 == r.1) {
            return None;
        }
        let tmpl_inner = string_literal_inner(&inner[ranges[msg_idx].0..ranges[msg_idx].1])?;
        if !template_has_placeholder(tmpl_inner) {
            return None;
        }
        let args: Vec<String> = ranges
            .iter()
            .map(|&(s, e)| self.rewrite_range(&inner[s..e], base + open + 1 + s, false))
            .collect();
        let original = &text[start..close + 1];
        let body = match convert_template_args(tmpl_inner, &args[msg_idx + 1..]) {
            Ok(b) => b,
            Err(e) => {
                self.record(
                    "W0008",
                    base + start,
                    original,
                    None,
                    format!("`{name}` with a `{{}}` format string: use `${{..}}` interpolation ({e})"),
                );
                return None;
            }
        };
        let mut parts: Vec<String> = args[..msg_idx].to_vec();
        parts.push(format!("\"{body}\""));
        let rep = format!("{name}({})", parts.join(", "));
        self.record(
            "W0008",
            base + start,
            original,
            Some(rep.clone()),
            format!("`{name}` with a `{{}}` format string: use `${{..}}` interpolation"),
        );
        Some((rep, close + 1))
    }
}

fn inner_has_top_level_semicolon(inner: &str) -> bool {
    split_top_level(inner, b";").len() > 1
}

fn convert_template_args(tmpl_inner: &str, rest: &[String]) -> Result<String, String> {
    let mut positional = Vec::new();
    let mut named = Vec::new();
    for a in rest {
        if let Some((n, e)) = split_named_arg(a) {
            named.push((n, e));
        } else {
            positional.push(a.clone());
        }
    }
    if positional.iter().chain(named.iter().map(|n| &n.1)).any(|a| {
        a.contains("//") || a.contains("/*")
    }) {
        return Err("argument contains a comment".to_string());
    }
    if positional.is_empty() && named.is_empty() && !template_has_placeholder(tmpl_inner) {
        // plain text: `{{` -> `{`, escape `$`
        let plain = unescape_braces(tmpl_inner);
        return rust_template_to_interpolation(&plain.replace('{', "{{").replace('}', "}}"), &[], &[]);
    }
    rust_template_to_interpolation(tmpl_inner, &positional, &named)
}

fn prev_word(text: &str, i: usize) -> Option<&str> {
    let head = text[..i].trim_end();
    let b = head.as_bytes();
    let mut s = b.len();
    while s > 0 && is_ident_char(b[s - 1]) {
        s -= 1;
    }
    if s == b.len() {
        None
    } else {
        Some(&head[s..])
    }
}

/// Is the `vec![..]` at `i` in a position where the list type is determined by context
/// (so `[..]` is guaranteed to lower to the same `Vec`)? `let x = `, `field: `, `for p in `.
fn vec_context_ok(text: &str, i: usize) -> bool {
    let head = text[..i].trim_end();
    if prev_word(text, i) == Some("in") {
        return true;
    }
    if let Some(stripped) = head.strip_suffix('=') {
        let before = stripped.chars().last();
        if matches!(
            before,
            Some('=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^')
        ) {
            return false;
        }
        // statement must be a `let`
        let line_start = stripped.rfind('\n').map(|p| p + 1).unwrap_or(0);
        let stmt = stripped[line_start..].trim_start();
        return stmt.starts_with("let ");
    }
    if let Some(stripped) = head.strip_suffix(':') {
        return !stripped.ends_with(':')
            && stripped
                .trim_end()
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            && inside_braces_not_let(stripped);
    }
    false
}

/// `field: vec![..]`: the `:` belongs to a struct literal / pattern, not a `let x: T`
/// annotation or a function parameter list.
fn inside_braces_not_let(head: &str) -> bool {
    let line_start = head.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let line = head[line_start..].trim_start();
    !(line.starts_with("let ") || line.starts_with("fn ") || line.starts_with("pub fn "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(s: &str) -> String {
        fix_source(s).text
    }

    #[test]
    fn println_conversion() {
        assert_eq!(
            fx(r#"println!("a {} b {:?}", x, y)"#),
            r#"println("a ${x} b ${y:?}")"#
        );
        assert_eq!(fx(r#"println!("hi")"#), r#"println("hi")"#);
        assert_eq!(fx(r#"println!()"#), r#"println()"#);
        assert_eq!(fx(r#"println!("{{}} {:.2}", v)"#), r#"println("{} ${v:.2}")"#);
        assert_eq!(fx(r#"print!("{x}")"#), r#"print("${x}")"#);
    }

    #[test]
    fn streams_and_misc() {
        assert_eq!(fx(r#"eprintln!("e {}", x)"#), r#"stderr.println("e ${x}")"#);
        assert_eq!(fx(r#"eprint!("e")"#), r#"stderr.print("e")"#);
        assert_eq!(fx("dbg!(a.b)"), "debug(a.b)");
        assert_eq!(fx(r#"let s = format!("v={}", x)"#), r#"let s = "v=${x}""#);
        assert_eq!(fx(r#"panic!("bad {}", n)"#), r#"panic("bad ${n}")"#);
        assert_eq!(fx("todo!()"), "todo()");
        assert_eq!(fx("unreachable!()"), "unreachable()");
        assert_eq!(fx("assert!(a == b)"), "assert(a == b)");
        assert_eq!(fx(r#"assert!(a, "m {}", b)"#), r#"assert(a, "m ${b}")"#);
        assert_eq!(fx(r#"assert_eq!(a, b, "m {}", c)"#), r#"assert_eq(a, b, "m ${c}")"#);
        assert_eq!(fx("assert_ne!(a, b)"), "assert_ne(a, b)");
    }

    #[test]
    fn nested_and_multiline() {
        assert_eq!(
            fx("println!(\"{}\", format!(\"[{}]\", x))"),
            "println(\"${(\"[${x}]\")}\")"
        );
        let src = "println!(\n    \"a {} b {}\",\n    foo(1, 2),\n    bar,\n)";
        assert_eq!(fx(src), "println(\"a ${foo(1, 2)} b ${bar}\")");
    }

    #[test]
    fn vec_rules() {
        assert_eq!(fx("let v = vec![1, 2, 3]"), "let v = [1, 2, 3]");
        assert_eq!(fx("let mut v = vec![]"), "let mut v = []");
        assert_eq!(fx("foo(vec![])"), "foo([])");
        assert_eq!(fx("let g = vec![vec![0; 3]; 3]"), "let g = [[0; 3]; 3]");
        // argument position is type dependent: kept
        assert_eq!(fx("foo(vec![1, 2])"), "foo(vec![1, 2])");
        assert_eq!(fx("x = vec![1]"), "x = vec![1]");
        assert_eq!(fx("for p in vec![1, 2] {"), "for p in [1, 2] {");
        assert_eq!(fx("S { items: vec![1] }"), "S { items: [1] }");
        assert_eq!(fx("let v: Vec<int> = vec![1]"), "let v: Vec<int> = [1]");
    }

    #[test]
    fn legacy_plain_templates() {
        assert_eq!(fx(r#"println("a {}", x)"#), r#"println("a ${x}")"#);
        assert_eq!(fx(r#"println("{ \"k\": 1 }")"#), r#"println("{ \"k\": 1 }")"#);
        assert_eq!(fx(r#"assert(c, "m {}", x)"#), r#"assert(c, "m ${x}")"#);
    }

    #[test]
    fn leaves_strings_and_comments_alone() {
        let s = "// println!(\"{}\", x)\nlet a = \"println!(1)\"\n";
        assert_eq!(fx(s), s);
        assert_eq!(fx("x.println!(1)"), "x.println!(1)");
    }

    #[test]
    fn macros_in_interpolations() {
        assert_eq!(
            fx(r#"let a = "n=${format!("{}", n)}""#),
            r#"let a = "n=${("${n}")}""#
        );
    }

    #[test]
    fn idempotent_on_samples() {
        for s in [
            r#"println!("a {} b {:?}", x, y)"#,
            "let v = vec![vec![1], vec![2]]\nfoo(vec![3])",
            "println!(\"{}\", vec![1].len())",
            r#"eprintln!("{{}}")"#,
            r#"println!("$ {}", 1)"#,
        ] {
            let once = fx(s);
            assert_eq!(fx(&once), once, "not idempotent for {s}");
        }
    }

    #[test]
    fn markdown_fences() {
        let md = "text println!(\"{}\", x)\n```windjammer\nprintln!(\"{}\", x)\n```\n```rust\nprintln!(\"{}\", x);\n```\n";
        assert_eq!(
            fix_markdown(md),
            "text println!(\"{}\", x)\n```windjammer\nprintln(\"${x}\")\n```\n```rust\nprintln!(\"{}\", x);\n```\n"
        );
    }

    #[test]
    fn unconvertible_is_left_alone() {
        let s = r#"println!("{:w$}", x)"#;
        let out = fix_source(s);
        assert_eq!(out.text, s);
        assert_eq!(out.skipped_count(), 1);
    }

    #[test]
    fn findings_have_positions() {
        let out = fix_source("fn main() {\n    println!(\"{}\", x)\n}\n");
        assert_eq!(out.findings.len(), 1);
        assert_eq!((out.findings[0].line, out.findings[0].col), (2, 5));
        assert_eq!(out.findings[0].code, "W0008");
        assert_eq!(
            out.findings[0].replacement.as_deref(),
            Some("println(\"${x}\")")
        );
    }
}
