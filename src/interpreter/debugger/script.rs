//! The command language of `wj replay` (ADR-0035): one command per line, `#` starts a comment.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// `break LINE [if EXPR]` / `break FILE:LINE [if EXPR]`
    Break { file: Option<String>, line: usize, cond: Option<String> },
    Continue,
    /// `step [N]`
    Step(u64),
    /// `next`: run until a statement of this frame (or a caller) starts, stepping over calls.
    Next,
    /// `finish`: run until the current frame returns; reports the return value.
    Finish,
    /// `back [N]`
    Back(u64),
    /// `print NAME` or `print EXPR` (a read-only expression, evaluated in the stopped frame)
    Print(String),
    /// `watch NAME`: stop when the local changes
    Watch(String),
    Locals,
    Where,
}

/// A command and the text it was written as (error events quote it).
#[derive(Clone, Debug)]
pub struct Line {
    pub cmd: Cmd,
    pub text: String,
}

fn count(arg: Option<&str>, what: &str) -> Result<u64, String> {
    match arg {
        None => Ok(1),
        Some(a) => match a.parse::<u64>() {
            Ok(n) if n > 0 => Ok(n),
            _ => Err(format!("`{what}` takes a positive count, found `{a}`")),
        },
    }
}

pub(crate) fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(ch) if ch == '_' || ch.is_alphabetic()) && c.all(|ch| ch == '_' || ch.is_alphanumeric())
}

/// The line without its `#` comment (a `#` inside a string literal is text).
fn strip_comment(text: &str) -> &str {
    let (mut in_str, mut escaped) = (false, false);
    for (i, c) in text.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_str => escaped = true,
            '"' => in_str = !in_str,
            '#' if !in_str => return &text[..i],
            _ => {}
        }
    }
    text
}

/// At most one word of argument.
fn one_arg<'a>(head: &str, rest: &'a str) -> Result<Option<&'a str>, String> {
    let mut words = rest.split_whitespace();
    let arg = words.next();
    match words.next() {
        Some(extra) => Err(format!("`{head}` takes at most one argument, found `{extra}` too")),
        None => Ok(arg),
    }
}

fn no_arg(head: &str, rest: &str, cmd: Cmd) -> Result<Cmd, String> {
    if rest.is_empty() {
        Ok(cmd)
    } else {
        Err(format!("`{head}` takes no argument"))
    }
}

/// `LINE`, `FILE:LINE`, each optionally followed by `if EXPR`.
fn parse_break(rest: &str) -> Result<Cmd, String> {
    let usage = "`break` takes a line: `break 12`, `break main.wj:12` or `break 12 if x > 3`";
    let (at, cond) = match rest.split_once(char::is_whitespace) {
        Some((a, c)) => (a, Some(c.trim())),
        None => (rest, None),
    };
    if at.is_empty() {
        return Err(usage.into());
    }
    let cond = match cond {
        None => None,
        Some(c) => match c.strip_prefix("if").filter(|e| e.starts_with(char::is_whitespace)).map(str::trim) {
            Some(e) if !e.is_empty() => Some(e.to_string()),
            _ => return Err(format!("`break {at}` may be followed by `if EXPR`, found `{c}`")),
        },
    };
    let (file, line) = match at.rsplit_once(':') {
        Some((f, l)) => (Some(f.to_string()), l),
        None => (None, at),
    };
    match line.parse::<usize>() {
        Ok(n) if n > 0 => Ok(Cmd::Break { file, line: n, cond }),
        _ => Err(format!("`break` takes a line number, found `{at}`")),
    }
}

pub fn parse_line(text: &str) -> Result<Option<Cmd>, String> {
    let body = strip_comment(text).trim();
    if body.is_empty() {
        return Ok(None);
    }
    let (head, rest) = match body.split_once(char::is_whitespace) {
        Some((h, r)) => (h, r.trim()),
        None => (body, ""),
    };
    let cmd = match head {
        "break" | "b" => parse_break(rest)?,
        "continue" | "c" => no_arg(head, rest, Cmd::Continue)?,
        "next" | "n" => no_arg(head, rest, Cmd::Next)?,
        "finish" => no_arg(head, rest, Cmd::Finish)?,
        "step" | "s" => Cmd::Step(count(one_arg(head, rest)?, "step")?),
        "back" => Cmd::Back(count(one_arg(head, rest)?, "back")?),
        "print" | "p" if rest.is_empty() => return Err("`print` takes a binding name or a read-only expression".into()),
        "print" | "p" => Cmd::Print(rest.to_string()),
        "watch" => match one_arg(head, rest)? {
            Some(n) if is_name(n) => Cmd::Watch(n.to_string()),
            Some(n) => return Err(format!("`watch` takes the name of a local, found `{n}`")),
            None => return Err("`watch` takes the name of a local".into()),
        },
        "locals" => no_arg(head, rest, Cmd::Locals)?,
        "where" | "bt" => no_arg(head, rest, Cmd::Where)?,
        other => return Err(format!("unknown command `{other}` (known: break, continue, step, next, finish, back, print, watch, locals, where)")),
    };
    Ok(Some(cmd))
}

/// The whole script; the first bad line is an error naming its line number.
pub fn parse(text: &str) -> Result<Vec<Line>, String> {
    let mut out = Vec::new();
    for (i, t) in text.lines().enumerate() {
        match parse_line(t) {
            Ok(Some(cmd)) => out.push(Line { cmd, text: t.trim().to_string() }),
            Ok(None) => {}
            Err(m) => return Err(format!("script line {}: {m}", i + 1)),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brk(file: Option<&str>, line: usize, cond: Option<&str>) -> Option<Cmd> {
        Some(Cmd::Break { file: file.map(String::from), line, cond: cond.map(String::from) })
    }

    #[test]
    fn commands_parse() {
        assert_eq!(parse_line("break 12").unwrap(), brk(None, 12, None));
        assert_eq!(parse_line("  b src/a.wj:3 # here").unwrap(), brk(Some("src/a.wj"), 3, None));
        assert_eq!(parse_line("break 10 if sum > 5 && n != 3").unwrap(), brk(None, 10, Some("sum > 5 && n != 3")));
        assert_eq!(parse_line("step").unwrap(), Some(Cmd::Step(1)));
        assert_eq!(parse_line("step 4").unwrap(), Some(Cmd::Step(4)));
        assert_eq!(parse_line("next").unwrap(), Some(Cmd::Next));
        assert_eq!(parse_line("finish").unwrap(), Some(Cmd::Finish));
        assert_eq!(parse_line("back 2").unwrap(), Some(Cmd::Back(2)));
        assert_eq!(parse_line("print total").unwrap(), Some(Cmd::Print("total".into())));
        assert_eq!(parse_line("print xs.len() + 1").unwrap(), Some(Cmd::Print("xs.len() + 1".into())));
        assert_eq!(parse_line("print s == \"a#b\" # note").unwrap(), Some(Cmd::Print("s == \"a#b\"".into())));
        assert_eq!(parse_line("watch sum").unwrap(), Some(Cmd::Watch("sum".into())));
        assert_eq!(parse_line("where").unwrap(), Some(Cmd::Where));
        assert_eq!(parse_line("# only a comment").unwrap(), None);
        assert_eq!(parse_line("").unwrap(), None);
    }

    #[test]
    fn bad_commands_are_refused() {
        let bad = [
            "jump 3", "break", "break x", "break 0", "break 3 when x", "break 3 if", "step 0", "back -1", "print", "where now", "next 2", "finish now", "watch",
            "watch a.b", "watch a b",
        ];
        for b in bad {
            assert!(parse_line(b).is_err(), "{b} should be refused");
        }
        let e = parse("step\nfly\n").unwrap_err();
        assert!(e.starts_with("script line 2:"), "{e}");
    }
}
