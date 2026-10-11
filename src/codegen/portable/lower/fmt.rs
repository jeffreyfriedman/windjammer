//! Format-string parsing (`{}`, `{:?}`, `{:>8.3}`, `{{`) into IR pieces.

use super::{Lowerer, PResult};
use crate::codegen::portable::ir::*;
use crate::parser::Expression;

/// Split a format string into literal text and (arg selector, spec) placeholders.
pub(super) enum Raw {
    Lit(String),
    Hole { selector: Option<String>, spec: FmtSpec },
}

/// Parse the part of a `{:spec}` after the colon. Anything the portable backends cannot render
/// faithfully (sign `+`, alternate `#` forms, `E`, `x?`) is an error, never silently ignored.
pub(super) fn parse_spec(s: &str) -> Result<FmtSpec, String> {
    let mut spec = FmtSpec::default();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    if chars.len() >= 2 && matches!(chars[1], '<' | '>' | '^') {
        spec.fill = Some(chars[0]);
        spec.align = Some(chars[1]);
        i = 2;
    } else if !chars.is_empty() && matches!(chars[0], '<' | '>' | '^') {
        spec.align = Some(chars[0]);
        i = 1;
    }
    if matches!(chars.get(i), Some('+' | '-')) {
        return Err(format!("format flag `{}` in `{{:{s}}}`", chars[i]));
    }
    if chars.get(i) == Some(&'#') && chars[i..].iter().collect::<String>() != "#?" {
        return Err(format!("alternate `#` format in `{{:{s}}}`"));
    }
    if i < chars.len() && chars[i] == '0' {
        spec.zero_pad = true;
        i += 1;
    }
    let mut width = String::new();
    while i < chars.len() && chars[i].is_ascii_digit() {
        width.push(chars[i]);
        i += 1;
    }
    spec.width = width.parse().ok();
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        let mut p = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            p.push(chars[i]);
            i += 1;
        }
        spec.precision = p.parse().ok();
    }
    let rest: String = chars[i..].iter().collect();
    match rest.as_str() {
        "" => {}
        "?" => spec.debug = true,
        "#?" => {
            spec.debug = true;
            spec.pretty = true;
        }
        "x" | "X" | "b" | "o" | "e" => spec.radix = rest.chars().next(),
        other => return Err(format!("format type `{other}` in `{{:{s}}}`")),
    }
    Ok(spec)
}

pub(super) fn parse_format(fmt: &str) -> Result<Vec<Raw>, String> {
    let mut out = Vec::new();
    let mut lit = String::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' if chars.get(i + 1) == Some(&'{') => {
                lit.push('{');
                i += 2;
            }
            '}' if chars.get(i + 1) == Some(&'}') => {
                lit.push('}');
                i += 2;
            }
            '{' => {
                let end = chars[i..].iter().position(|c| *c == '}').map(|p| p + i).unwrap_or(chars.len() - 1);
                let inner: String = chars[i + 1..end].iter().collect();
                if !lit.is_empty() {
                    out.push(Raw::Lit(std::mem::take(&mut lit)));
                }
                let (sel, spec) = match inner.split_once(':') {
                    Some((a, b)) => (a.to_string(), parse_spec(b)?),
                    None => (inner.clone(), FmtSpec::default()),
                };
                out.push(Raw::Hole { selector: if sel.is_empty() { None } else { Some(sel) }, spec });
                i = end + 1;
            }
            c => {
                lit.push(c);
                i += 1;
            }
        }
    }
    if !lit.is_empty() {
        out.push(Raw::Lit(lit));
    }
    Ok(out)
}

impl<'e> Lowerer<'e> {
    /// Lower `fmt, args...` into pieces. `fmt` must be a string literal.
    pub(super) fn format_pieces(&mut self, fmt: &str, args: &[&Expression]) -> PResult<Vec<FmtPiece>> {
        let mut pieces = Vec::new();
        let mut next = 0;
        let raws = match parse_format(fmt) {
            Ok(r) => r,
            Err(e) => return self.err(format!("unsupported {e}")),
        };
        for raw in raws {
            match raw {
                Raw::Lit(s) => pieces.push(FmtPiece::Lit(s)),
                Raw::Hole { selector, spec } => {
                    let arg = match selector.as_deref() {
                        None => {
                            next += 1;
                            args.get(next - 1).map(|e| self.expr(e, None)).transpose()?
                        }
                        Some(s) if s.chars().all(|c| c.is_ascii_digit()) => {
                            args.get(s.parse::<usize>().unwrap_or(usize::MAX)).map(|e| self.expr(e, None)).transpose()?
                        }
                        Some(name) => Some(self.local_expr(name)?),
                    };
                    let Some(arg) = arg else { return self.err("format placeholder without argument") };
                    pieces.push(FmtPiece::Arg(arg, spec));
                }
            }
        }
        Ok(pieces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_holes_and_escapes() {
        let raw = parse_format("a{{b}} {} {:?} {:>6.2}").unwrap();
        assert_eq!(raw.len(), 6);
        match &raw[3] {
            Raw::Hole { spec, .. } => assert!(spec.debug),
            _ => panic!(),
        }
        match &raw[5] {
            Raw::Hole { spec, .. } => {
                assert_eq!(spec.width, Some(6));
                assert_eq!(spec.precision, Some(2));
                assert_eq!(spec.align, Some('>'));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn unsupported_specs_are_errors_not_ignored() {
        for bad in ["+", "+5", "#x", "#08b", "E", "x?"] {
            assert!(parse_spec(bad).is_err(), "{bad} must be rejected");
        }
        assert!(parse_spec("#?").unwrap().pretty);
        let s = parse_spec("*^9.2").unwrap();
        assert_eq!((s.fill, s.align, s.width, s.precision), (Some('*'), Some('^'), Some(9), Some(2)));
        assert!(parse_spec("08b").unwrap().zero_pad);
    }
}
