//! Line layout: indentation levels derived from bracket nesting and continuation lines,
//! and rendering of each line.

use super::options::FormatOptions;
use super::spacing::space_between;
use super::tokens::Tok;

/// A source line: token range `start..end` and its computed indentation level.
#[derive(Debug, Clone)]
pub struct Line {
    pub start: usize,
    pub end: usize,
    pub indent: usize,
}

const CONTINUATION_STARTS: &[&str] = &[".", "|>", "|>?", "??", "&&", "||", "?"];
const CONTINUATION_ENDS: &[&str] = &[
    "=", "+", "-", "/", "%", "&&", "||", "|>", "|>?", "??", "==", "!=", "<=", ">=", "+=", "-=", "*=",
    "/=", "->", "=>",
];

pub fn layout(toks: &[Tok]) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut prev_last: Option<&Tok> = None;
    let mut s = 0;
    while s < toks.len() {
        let mut e = s + 1;
        while e < toks.len() && toks[e].nl == 0 {
            e += 1;
        }
        let comment_only = toks[s..e].iter().all(|t| t.is_comment());
        let pipe_next = comment_only && comment_precedes_pipe(&toks[e..]);
        let indent = line_indent(&toks[s..e], &mut stack, prev_last, lines.len(), pipe_next);
        lines.push(Line { start: s, end: e, indent });
        if !comment_only {
            prev_last = toks[s..e].iter().rev().find(|t| !t.is_comment());
        }
        s = e;
    }
    lines
}

/// Compute the indent of a line and update the open-bracket stack with its tokens.
fn line_indent(
    line: &[Tok],
    stack: &mut Vec<(usize, usize)>,
    prev_last: Option<&Tok>,
    line_no: usize,
    pipe_next: bool,
) -> usize {
    // (A line that starts with a block comment but has code after it is laid out as code.)
    if line.iter().all(|t| t.is_comment()) {
        return stack.last().map_or(0, |e| e.0 + 1) + usize::from(pipe_next);
    }
    let leading = line.iter().take_while(|t| t.is_closer()).count();
    let mut popped = None;
    for _ in 0..leading {
        popped = stack.pop();
    }
    let indent = if leading > 0 {
        popped.map_or(0, |e| e.0)
    } else {
        let opened_by_prev = stack.last().is_some_and(|e| e.1 + 1 == line_no);
        let cont = is_continuation(line, prev_last) && !(opened_by_prev && line[0].is("."));
        stack.last().map_or(0, |e| e.0 + 1) + usize::from(cont)
    };
    for t in &line[leading..] {
        if t.is_opener() {
            stack.push((indent, line_no));
        } else if t.is_closer() {
            stack.pop();
        }
    }
    indent
}

/// A comment line sitting between two pipe stages is indented like the stages around it.
fn comment_precedes_pipe(rest: &[Tok]) -> bool {
    let mut line_starts = rest.iter().filter(|t| t.nl > 0);
    line_starts.find(|t| !t.is_comment()).is_some_and(|t| t.is("|>") || t.is("|>?"))
}

fn is_continuation(line: &[Tok], prev_last: Option<&Tok>) -> bool {
    let starts = CONTINUATION_STARTS.iter().any(|s| line[0].is(s));
    let follows = prev_last.is_some_and(|t| CONTINUATION_ENDS.iter().any(|s| t.is(s)));
    starts || follows
}

/// Render one line (indentation included, no trailing newline).
pub fn render_line(toks: &[Tok], line: &Line, opts: &FormatOptions) -> String {
    let mut out = " ".repeat(line.indent * opts.indent);
    for i in line.start..line.end {
        if i > line.start && space_between(toks, i, line.start) {
            out.push(' ');
        }
        out.push_str(&toks[i].text);
    }
    out
}

/// Assemble rendered lines, inserting at most one blank line where the source had any.
pub fn assemble(toks: &[Tok], lines: &[Line], rendered: &[String]) -> String {
    let mut out = String::new();
    for (line, text) in lines.iter().zip(rendered) {
        if !out.is_empty() && toks[line.start].nl >= 2 {
            out.push('\n');
        }
        out.push_str(text);
        out.push('\n');
    }
    out
}
