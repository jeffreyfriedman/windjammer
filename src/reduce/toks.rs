//! Token documents: lossless token vectors, rendering, bracket groups and "units" (the deletable
//! pieces inside a group). Built on the formatter's lossless scanner so comments survive until the
//! reducer strips them deliberately.

use crate::formatter::{scan_lossless, Tok, TokKind};
use std::ops::Range;

pub fn scan(src: &str) -> Option<Vec<Tok>> {
    scan_lossless(src).ok()
}

/// Render tokens back to text: author line breaks are kept, other gaps become a single space.
pub fn render(toks: &[Tok]) -> String {
    let mut out = String::new();
    for (i, t) in toks.iter().enumerate() {
        if i > 0 && t.nl == 0 {
            out.push(' ');
        } else {
            for _ in 0..t.nl.min(2) {
                out.push('\n');
            }
        }
        out.push_str(&t.text);
    }
    out.push('\n');
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GroupKind {
    /// The whole file (open/close are virtual).
    Root,
    Block,
    Paren,
    Bracket,
    Angle,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub kind: GroupKind,
    /// Index of the opener token (`usize::MAX` for the root).
    pub open: usize,
    /// Index of the closer token (`toks.len()` for the root).
    pub close: usize,
    pub depth: usize,
}

impl Group {
    pub fn inner(&self) -> Range<usize> {
        let start = if self.kind == GroupKind::Root {
            0
        } else {
            self.open + 1
        };
        start..self.close
    }
}

const HEADER_KW: [&str; 5] = ["fn", "struct", "enum", "trait", "type"];

/// All bracket groups in document order, with nesting depth (root = 0). Angle groups are recognised
/// only in declaration headers (`fn f<T>`, `struct S<T>`, `impl<T>`), where `<` cannot be a comparison.
pub fn groups(toks: &[Tok]) -> Vec<Group> {
    let mut out = vec![Group {
        kind: GroupKind::Root,
        open: usize::MAX,
        close: toks.len(),
        depth: 0,
    }];
    let mut stack: Vec<usize> = Vec::new(); // indices into `out`
    let mut nest: Vec<u32> = Vec::new(); // plain `<` nesting inside each open angle group
    for (i, t) in toks.iter().enumerate() {
        if t.kind != TokKind::Punct {
            continue;
        }
        if let Some(&top) = stack.last() {
            if out[top].kind == GroupKind::Angle {
                let n = nest.last_mut().expect("parallel to stack");
                match t.text.as_str() {
                    "<" if !is_header_angle(toks, i) => {
                        *n += 1;
                        continue;
                    }
                    ">" if *n > 0 => {
                        *n -= 1;
                        continue;
                    }
                    ">>" if *n > 1 => {
                        *n -= 2;
                        continue;
                    }
                    ">>" if *n == 1 => *n = 0,
                    _ => {}
                }
            }
        }
        let kind = match t.text.as_str() {
            "{" => Some(GroupKind::Block),
            "(" => Some(GroupKind::Paren),
            "[" => Some(GroupKind::Bracket),
            "<" if is_header_angle(toks, i) => Some(GroupKind::Angle),
            _ => None,
        };
        if let Some(kind) = kind {
            out.push(Group {
                kind,
                open: i,
                close: toks.len(),
                depth: stack.len() + 1,
            });
            stack.push(out.len() - 1);
            nest.push(0);
            continue;
        }
        let closes = |k: GroupKind| match t.text.as_str() {
            "}" => k == GroupKind::Block,
            ")" => k == GroupKind::Paren,
            "]" => k == GroupKind::Bracket,
            ">" | ">>" => k == GroupKind::Angle,
            _ => false,
        };
        if let Some(&top) = stack.last() {
            if closes(out[top].kind) {
                out[top].close = i;
                stack.pop();
                nest.pop();
            }
        }
    }
    out.retain(|g| g.kind == GroupKind::Root || g.close < toks.len());
    out
}

fn is_header_angle(toks: &[Tok], i: usize) -> bool {
    if i == 0 || toks[i].space {
        return toks[i].space && i > 0 && toks[i - 1].text == "impl";
    }
    let prev = &toks[i - 1];
    if prev.kind != TokKind::Word {
        return false;
    }
    if prev.text == "impl" {
        return true;
    }
    i >= 2 && toks[i - 2].kind == TokKind::Word && HEADER_KW.contains(&toks[i - 2].text.as_str())
}

fn continues_line(prev: &Tok, next: &Tok) -> bool {
    const CONT_START: [&str; 21] = [
        ".", "|>", "|>?", "&&", "||", "??", "?", "=>", "->", "::", "==", "!=", "+", "/", "%", "^",
        "|", "{", ">=", "<=", ">",
    ];
    const CONT_END: [&str; 24] = [
        "=", "==", "!=", "+", "-", "*", "/", "%", "&&", "||", "|>", "|>?", ".", "::", "=>", "->",
        "<", ">", "<=", ">=", "??", ":", "+=", "-=",
    ];
    if next.kind == TokKind::Punct && CONT_START.contains(&next.text.as_str()) {
        return true;
    }
    if next.kind == TokKind::Word && matches!(next.text.as_str(), "else" | "where" | "as") {
        return true;
    }
    prev.kind == TokKind::Punct && CONT_END.contains(&prev.text.as_str())
}

/// Split a group's inner tokens into deletable units (ranges into `toks`, in order).
pub fn units(toks: &[Tok], g: &Group) -> Vec<Range<usize>> {
    let inner = g.inner();
    if inner.is_empty() {
        return Vec::new();
    }
    let comma_split = matches!(
        g.kind,
        GroupKind::Paren | GroupKind::Bracket | GroupKind::Angle
    );
    let mut out = Vec::new();
    let mut start = inner.start;
    let mut depth = 0i32;
    for i in inner.clone() {
        let t = &toks[i];
        if i > start && depth == 0 && !comma_split && t.nl > 0 && !continues_line(&toks[i - 1], t) {
            out.push(start..i);
            start = i;
        }
        if t.kind == TokKind::Punct {
            match t.text.as_str() {
                "(" | "[" | "{" => depth += 1,
                ")" | "]" | "}" => depth -= 1,
                "<" if g.kind == GroupKind::Angle => depth += 1,
                ">" if g.kind == GroupKind::Angle && depth > 0 => depth -= 1,
                ">>" if g.kind == GroupKind::Angle && depth > 1 => depth -= 2,
                _ => {}
            }
            if comma_split && depth == 0 && t.text == "," {
                out.push(start..i + 1);
                start = i + 1;
            }
        }
    }
    if start < inner.end {
        out.push(start..inner.end);
    }
    out
}

/// A copy of `toks` without the given (sorted, disjoint) ranges.
pub fn without_ranges(toks: &[Tok], ranges: &[Range<usize>]) -> Vec<Tok> {
    let mut out = Vec::with_capacity(toks.len());
    let mut r = ranges.iter().peekable();
    for (i, t) in toks.iter().enumerate() {
        while r.peek().is_some_and(|x| x.end <= i) {
            r.next();
        }
        if r.peek().is_some_and(|x| x.start <= i) {
            continue;
        }
        out.push(t.clone());
    }
    out
}

/// Replace `range` by `repl` (the first replacement token takes the old first token's line break).
pub fn replace_range(toks: &[Tok], range: Range<usize>, repl: Vec<Tok>) -> Vec<Tok> {
    let mut out = Vec::with_capacity(toks.len());
    out.extend_from_slice(&toks[..range.start]);
    for (k, mut t) in repl.into_iter().enumerate() {
        if k == 0 {
            t.nl = toks.get(range.start).map(|x| x.nl).unwrap_or(0);
        }
        out.push(t);
    }
    out.extend_from_slice(&toks[range.end..]);
    out
}

pub fn word(text: &str) -> Tok {
    Tok::new(TokKind::Word, text)
}

pub fn string_lit() -> Tok {
    Tok::new(TokKind::Str, "\"\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        scan(s).unwrap()
    }

    #[test]
    fn render_round_trips_through_scan() {
        let src = "fn f(a: int) -> int {\n    let x = \"s ${a}\"\n    a + 1\n}\n";
        let t = toks(src);
        let again = toks(&render(&t));
        let texts = |v: &[Tok]| v.iter().map(|t| t.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(&t), texts(&again));
    }

    #[test]
    fn block_units_split_on_lines_and_keep_continuations() {
        let t = toks("fn f() {\n    let a = x\n        .y()\n    let b = 2\n}\n");
        let gs = groups(&t);
        let body = gs.iter().find(|g| g.kind == GroupKind::Block).unwrap();
        assert_eq!(units(&t, body).len(), 2);
    }

    #[test]
    fn paren_units_split_on_commas_and_header_angles_are_groups() {
        let t = toks("fn f<T, U>(a: Vec<int>, b: int) {}\n");
        let gs = groups(&t);
        let paren = gs.iter().find(|g| g.kind == GroupKind::Paren).unwrap();
        assert_eq!(units(&t, paren).len(), 2);
        let angle = gs.iter().find(|g| g.kind == GroupKind::Angle).unwrap();
        assert_eq!(units(&t, angle).len(), 2);
    }

    #[test]
    fn without_ranges_removes_exactly_those_tokens() {
        let t = toks("a b c d e\n");
        let r = without_ranges(&t, &[1..2, 3..4]);
        assert_eq!(
            r.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
            ["a", "c", "e"]
        );
    }
}
