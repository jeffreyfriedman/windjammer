//! The command language of `wj replay` (ADR-0035): one command per line, `#` starts a comment.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// `break LINE` / `break FILE:LINE`
    Break { file: Option<String>, line: usize },
    Continue,
    /// `step [N]`
    Step(u64),
    /// `back [N]`
    Back(u64),
    /// `print NAME`
    Print(String),
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

fn is_name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(ch) if ch == '_' || ch.is_alphabetic()) && c.all(|ch| ch == '_' || ch.is_alphanumeric())
}

pub fn parse_line(text: &str) -> Result<Option<Cmd>, String> {
    let body = text.split('#').next().unwrap_or("").trim();
    if body.is_empty() {
        return Ok(None);
    }
    let mut words = body.split_whitespace();
    let head = words.next().unwrap_or("");
    let arg = words.next();
    if let Some(extra) = words.next() {
        return Err(format!("`{head}` takes at most one argument, found `{extra}` too"));
    }
    let cmd = match head {
        "break" | "b" => {
            let a = arg.ok_or("`break` takes a line: `break 12` or `break main.wj:12`")?;
            let (file, line) = match a.rsplit_once(':') {
                Some((f, l)) => (Some(f.to_string()), l),
                None => (None, a),
            };
            match line.parse::<usize>() {
                Ok(n) if n > 0 => Cmd::Break { file, line: n },
                _ => return Err(format!("`break` takes a line number, found `{a}`")),
            }
        }
        "continue" | "c" => Cmd::Continue,
        "step" | "s" => Cmd::Step(count(arg, "step")?),
        "back" => Cmd::Back(count(arg, "back")?),
        "print" | "p" => match arg {
            Some(n) if is_name(n) => Cmd::Print(n.to_string()),
            Some(n) => return Err(format!("`print` takes the name of a binding, found `{n}`")),
            None => return Err("`print` takes the name of a binding".into()),
        },
        "locals" => Cmd::Locals,
        "where" | "bt" => Cmd::Where,
        other => return Err(format!("unknown command `{other}` (known: break, continue, step, back, print, locals, where)")),
    };
    if arg.is_some() && matches!(cmd, Cmd::Continue | Cmd::Locals | Cmd::Where) {
        return Err(format!("`{head}` takes no argument"));
    }
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

    #[test]
    fn commands_parse() {
        assert_eq!(parse_line("break 12").unwrap(), Some(Cmd::Break { file: None, line: 12 }));
        assert_eq!(parse_line("  b src/a.wj:3 # here").unwrap(), Some(Cmd::Break { file: Some("src/a.wj".into()), line: 3 }));
        assert_eq!(parse_line("step").unwrap(), Some(Cmd::Step(1)));
        assert_eq!(parse_line("step 4").unwrap(), Some(Cmd::Step(4)));
        assert_eq!(parse_line("back 2").unwrap(), Some(Cmd::Back(2)));
        assert_eq!(parse_line("print total").unwrap(), Some(Cmd::Print("total".into())));
        assert_eq!(parse_line("where").unwrap(), Some(Cmd::Where));
        assert_eq!(parse_line("# only a comment").unwrap(), None);
        assert_eq!(parse_line("").unwrap(), None);
    }

    #[test]
    fn bad_commands_are_refused() {
        for bad in ["jump 3", "break", "break x", "break 0", "step 0", "back -1", "print", "print a.b", "where now", "print a b"] {
            assert!(parse_line(bad).is_err(), "{bad} should be refused");
        }
        let e = parse("step\nfly\n").unwrap_err();
        assert!(e.starts_with("script line 2:"), "{e}");
    }
}
