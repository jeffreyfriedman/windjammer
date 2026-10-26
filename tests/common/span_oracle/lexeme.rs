//! Independent lexeme oracle: what source text a token may have been produced from.
//! Written from the language's documented lexical grammar, not from `src/lexer`.

use windjammer::lexer::Token;

/// `Ok(())` when `lexeme` is a legal spelling of `tok`; `Err(why)` otherwise.
pub fn check_lexeme(tok: &Token, lexeme: &str, at_eof: bool) -> Result<(), String> {
    use Token::*;
    if let Some(fixed) = fixed_text(tok) {
        return if lexeme == fixed { Ok(()) } else { Err(format!("expected {fixed:?}")) };
    }
    match tok {
        Ident(name) => eq(lexeme, name),
        BoolLiteral(b) => eq(lexeme, if *b { "true" } else { "false" }),
        Lifetime(n) => eq(lexeme, &format!("'{n}")),
        Decorator(n) => eq(lexeme, &format!("@{n}")),
        Newline => eq(lexeme, "\n"),
        // The payload is a diagnostic message; the span must cover the offending text.
        Invalid(_) => (!lexeme.is_empty()).then_some(()).ok_or_else(|| "invalid token with empty span".to_string()),
        Eof => eq(lexeme, ""),
        DocComment(content) => {
            let rest = lexeme.strip_prefix("///").ok_or("doc comment must start with ///")?;
            if rest.contains('\n') {
                return Err("doc comment lexeme spans a newline".into());
            }
            let body = rest.strip_prefix(' ').unwrap_or(rest);
            eq(body, content)
        }
        IntLiteral(n) => {
            let (v, _) = parse_int(lexeme)?;
            let want = *n as i128;
            if v == want || (*n == i64::MIN && v == 1i128 << 63) {
                Ok(())
            } else {
                Err(format!("lexeme value {v} != token value {n}"))
            }
        }
        IntLiteralSuffixed(n, suffix) => {
            let (v, text_suffix) = parse_int(lexeme)?;
            match text_suffix {
                Some(s) if s == *suffix => (v as u64 as i64 == *n || v == *n as i128)
                    .then_some(())
                    .ok_or_else(|| format!("lexeme value {v} != token value {n}")),
                Some(s) => Err(format!("suffix {s:?} != token suffix {suffix:?}")),
                // An unsuffixed literal above i64::MAX (any radix) is typed u64 by the lexer.
                None if v > i64::MAX as i128 => (suffix == "u64" && v as u64 as i64 == *n)
                    .then_some(())
                    .ok_or_else(|| "literal above i64::MAX must be u64 bits".to_string()),
                None => Err("suffixed token but lexeme has no suffix".into()),
            }
        }
        FloatLiteral(f) => {
            let cleaned: std::string::String = lexeme.chars().filter(|c| *c != '_').collect();
            let parsed: f64 = cleaned.parse().map_err(|_| format!("{lexeme:?} is not a float"))?;
            if parsed.to_bits() == f.to_bits() {
                Ok(())
            } else {
                Err(format!("float value {parsed} != {f}"))
            }
        }
        StringLiteral(s) => {
            if lexeme.starts_with("r#") {
                let hashes = lexeme[1..].chars().take_while(|c| *c == '#').count();
                let open = 1 + hashes + 1;
                let close = format!("\"{}", "#".repeat(hashes));
                let body = lexeme.get(open..).ok_or("short raw string")?;
                let body = match body.strip_suffix(close.as_str()) {
                    Some(b) => b,
                    None if at_eof => body,
                    None => return Err("raw string not terminated".into()),
                };
                return eq(body, s);
            }
            let body = lexeme.strip_prefix('"').ok_or("string must start with \"")?;
            let body = match body.strip_suffix('"') {
                Some(b) => b,
                None if at_eof => body,
                None => return Err("string not terminated by \"".into()),
            };
            eq(&decode_plain_string(body), s)
        }
        InterpolatedString(parts) => {
            let body = lexeme.strip_prefix('"').ok_or("string must start with \"")?;
            if !body.ends_with('"') && !at_eof {
                return Err("interpolated string not terminated".into());
            }
            for p in parts {
                if let windjammer::lexer::StringPart::Expression(e) = p {
                    if !lexeme.contains(&format!("${{{e}")) {
                        return Err(format!("interpolation {e:?} not found in lexeme"));
                    }
                }
            }
            if !lexeme.contains("${") {
                return Err("interpolated token without ${".into());
            }
            Ok(())
        }
        CharLiteral(c) => {
            let body = lexeme.strip_prefix('\'').ok_or("char must start with '")?;
            let body = body.strip_suffix('\'').unwrap_or(body);
            let decoded = match body.strip_prefix('\\') {
                Some(esc) => {
                    let mut it = esc.chars();
                    let e = it.next().ok_or("empty escape")?;
                    match e {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        other => other,
                    }
                }
                None => body.chars().next().ok_or("empty char literal")?,
            };
            if decoded == *c {
                Ok(())
            } else {
                Err(format!("char {decoded:?} != {c:?}"))
            }
        }
        // Everything else has fixed text and returned above. No wildcard on purpose: adding a
        // Token variant must break this match and force the oracle to learn its spelling.
        Fn | Let | Mut | Const | Static | Struct | Enum | Trait | Impl | Match | If | Else | For
        | In | While | Loop | Return | Break | Continue | Use | Mod | Extern | Thread | Async
        | Await | Defer | Pub | Self_ | Unsafe | As | Where | Type | Dyn | Bound | Int | Int32
        | Uint | Float | Bool | String | Plus | Minus | Star | Slash | Percent | Eq | Ne | Lt
        | Le | Gt | Ge | And | Or | Not | Caret | Shl | Shr | Assign | PlusAssign | MinusAssign
        | StarAssign | SlashAssign | PercentAssign | AndAssign | OrAssign | XorAssign
        | ShlAssign | ShrAssign | Arrow | LeftArrow | FatArrow | At | LParen | RParen | LBrace
        | RBrace | LBracket | RBracket | Comma | Dot | DotDot | DotDotEq | Colon | ColonColon
        | Semicolon | Question | Ampersand | Pipe | PipeOp | PipeTry | Underscore | Bang => {
            Err(format!("{tok:?} missing from fixed_text table"))
        }
    }
}

fn eq(got: &str, want: &str) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("lexeme {got:?} != expected {want:?}"))
    }
}

/// Independent decoding of the documented escapes of a non-raw, non-interpolated string body.
fn decode_plain_string(body: &str) -> String {
    let mut out = String::new();
    let mut it = body.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' => match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(o) => out.push(o),
                None => {}
            },
            '$' if it.peek() == Some(&'$') => {
                it.next();
                out.push('$');
            }
            '$' if it.peek() == Some(&'{') => {
                // `${"` is a literal dollar-brace in the lexer's grammar.
                out.push('$');
                out.push('{');
                it.next();
                if it.peek() == Some(&'"') {
                    // the quote ends the string body in the real lexeme; nothing more to do
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// (value, suffix text) of an integer lexeme in any radix, `_` separators allowed.
fn parse_int(lexeme: &str) -> Result<(i128, Option<String>), String> {
    let (radix, digits) = match lexeme.get(..2) {
        Some("0x") | Some("0X") => (16, &lexeme[2..]),
        Some("0b") | Some("0B") => (2, &lexeme[2..]),
        Some("0o") | Some("0O") => (8, &lexeme[2..]),
        _ => (10, lexeme),
    };
    let mut num = String::new();
    let mut suffix = String::new();
    for c in digits.chars() {
        if !suffix.is_empty() {
            suffix.push(c);
        } else if c == '_' {
        } else if c.is_digit(radix) {
            num.push(c);
        } else {
            suffix.push(c);
        }
    }
    let v = u128::from_str_radix(&num, radix).map_err(|e| format!("{lexeme:?}: {e}"))?;
    Ok((v as i128, (!suffix.is_empty()).then_some(suffix)))
}

/// Exact spelling of every token with a fixed spelling.
pub fn fixed_text(tok: &Token) -> Option<&'static str> {
    use Token::*;
    Some(match tok {
        Fn => "fn",
        Let => "let",
        Mut => "mut",
        Const => "const",
        Static => "static",
        Struct => "struct",
        Enum => "enum",
        Trait => "trait",
        Impl => "impl",
        Match => "match",
        If => "if",
        Else => "else",
        For => "for",
        In => "in",
        While => "while",
        Loop => "loop",
        Return => "return",
        Break => "break",
        Continue => "continue",
        Use => "use",
        Mod => "mod",
        Extern => "extern",
        Thread => "thread",
        Async => "async",
        Await => "await",
        Defer => "defer",
        Pub => "pub",
        Self_ => "self",
        Unsafe => "unsafe",
        As => "as",
        Where => "where",
        Type => "type",
        Dyn => "dyn",
        Bound => "bound",
        Int => "int",
        Int32 => "int32",
        Uint => "uint",
        Float => "float",
        Bool => "bool",
        String => "string",
        Plus => "+",
        Minus => "-",
        Star => "*",
        Slash => "/",
        Percent => "%",
        Eq => "==",
        Ne => "!=",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        And => "&&",
        Or => "||",
        Not => "!",
        Caret => "^",
        Shl => "<<",
        Shr => ">>",
        Assign => "=",
        PlusAssign => "+=",
        MinusAssign => "-=",
        StarAssign => "*=",
        SlashAssign => "/=",
        PercentAssign => "%=",
        AndAssign => "&=",
        OrAssign => "|=",
        XorAssign => "^=",
        ShlAssign => "<<=",
        ShrAssign => ">>=",
        Arrow => "->",
        LeftArrow => "<-",
        FatArrow => "=>",
        At => "@",
        LParen => "(",
        RParen => ")",
        LBrace => "{",
        RBrace => "}",
        LBracket => "[",
        RBracket => "]",
        Comma => ",",
        Dot => ".",
        DotDot => "..",
        DotDotEq => "..=",
        Colon => ":",
        ColonColon => "::",
        Semicolon => ";",
        Question => "?",
        Ampersand => "&",
        Pipe => "|",
        PipeOp => "|>",
        PipeTry => "|>?",
        Underscore => "_",
        Bang => "!",
        _ => return None,
    })
}
