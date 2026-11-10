//! When is `x.clone()` NOT redundant? (W0005 precision.)
//!
//! W0005 asks the author to delete an explicit `.clone()` because "the compiler copies values when they are used again". That holds
//! only where the compiler really inserts the copy. Where it does not (a `string` moved into an `if` branch inside a loop is the
//! recorded case, `tests/lint_clone_need_test.rs`), the clone is the only thing between the program and rustc's E0382 (WJ0007), and
//! the warning would send the author straight into the error. So the lint stays silent for a clone of a plain local that is
//! mentioned again after the clone, or anywhere in a loop that contains the clone (the next iteration is "after"): that is exactly
//! when removing the clone could turn it into a use after a move. It still fires for a clone whose receiver is not mentioned again
//! (a last use), and for receivers that are not plain locals (`items[0].clone()`, `self.name.clone()`).

use crate::lexer::{Lexer, StringPart, Token};

/// Is the `.clone()` whose expression starts at `line`/`column` (1-based) a clone of a local that is used again?
pub fn receiver_is_used_again(source: &str, line: usize, column: usize) -> bool {
    let toks = Lexer::new(source).tokenize_with_locations();
    // the `x . clone ( )` that the position falls in
    // the position may be the receiver's start or the method name (`clone`): either lies inside `x.clone()`
    let site = (0..toks.len().saturating_sub(4)).find(|&i| {
        (toks[i].line, toks[i].column) <= (line, column)
            && (line, column) <= (toks[i + 2].line, toks[i + 2].column)
            && matches!(&toks[i].token, Token::Ident(_))
            && toks[i + 1].token == Token::Dot
            && matches!(&toks[i + 2].token, Token::Ident(m) if m == "clone")
            && toks[i + 3].token == Token::LParen
            && toks[i + 4].token == Token::RParen
    });
    let Some(site) = site else { return false };
    // a plain local only: not `a.x.clone()`, not `(..).x.clone()`
    if site > 0 && toks[site - 1].token == Token::Dot {
        return false;
    }
    let Token::Ident(name) = &toks[site].token else { return false };
    let mentions = |t: &Token| match t {
        Token::Ident(s) => s == name,
        Token::InterpolatedString(parts) => parts.iter().any(|p| matches!(p, StringPart::Expression(e) if has_word(e, name))),
        _ => false,
    };
    // the enclosing braces, innermost last: (index of `{`, introduced by fn / a loop keyword)
    let mut stack: Vec<(usize, bool, bool)> = Vec::new();
    let (mut pending_fn, mut pending_loop) = (false, false);
    let mut enclosing: Vec<(usize, bool, bool)> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if i == site {
            enclosing = stack.clone();
        }
        match t.token {
            Token::Fn => pending_fn = true,
            Token::For | Token::While | Token::Loop => pending_loop = true,
            Token::Semicolon => {
                pending_fn = false;
                pending_loop = false;
            }
            Token::LBrace => {
                stack.push((i, pending_fn, pending_loop));
                pending_fn = false;
                pending_loop = false;
            }
            Token::RBrace => {
                stack.pop();
            }
            _ => {}
        }
    }
    let close_of = |open: usize| {
        let mut depth = 0usize;
        for (j, t) in toks.iter().enumerate().skip(open) {
            match t.token {
                Token::LBrace => depth += 1,
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return j;
                    }
                }
                _ => {}
            }
        }
        toks.len()
    };
    // the function body that contains the clone (no function: nothing to say)
    let Some(&(fn_open, _, _)) = enclosing.iter().rev().find(|e| e.1) else { return false };
    let fn_close = close_of(fn_open);
    // used after the clone, anywhere in the function
    if toks[site + 5..fn_close].iter().any(|t| mentions(&t.token)) {
        return true;
    }
    // inside a loop: the next iteration is "after". A value declared inside the loop body before the clone is a new value each
    // time; any other receiver is used again by the clone itself on the next pass (without the clone it would move every time).
    for &(open, _, _) in enclosing.iter().filter(|e| e.2) {
        let declared_here = toks[open..site].windows(2).any(|w| {
            let named = |t: &Token| matches!(t, Token::Ident(s) if s == name);
            (matches!(w[0].token, Token::Let | Token::Mut | Token::For) && named(&w[1].token))
        });
        if !declared_here {
            return true;
        }
    }
    false
}

fn has_word(text: &str, word: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str) -> bool {
        // the position of the first `.clone()` receiver
        let (line, col) = src.lines().enumerate().find_map(|(l, t)| t.find(".clone()").map(|c| (l + 1, t[..c].rfind(|ch: char| !(ch.is_alphanumeric() || ch == '_')).map_or(0, |p| p + 1) + 1))).unwrap();
        receiver_is_used_again(src, line, col)
    }

    #[test]
    fn the_position_may_be_the_method_name() {
        let src = "fn f(a: string) -> int {\n    let b = a.clone()\n    take(b) + a.len()\n}\n";
        // line 2: `    let b = a.clone()`: `a` is column 13, `clone` column 15
        assert!(receiver_is_used_again(src, 2, 13));
        assert!(receiver_is_used_again(src, 2, 15));
    }

    #[test]
    fn a_clone_of_a_last_use_is_redundant() {
        assert!(!at("fn f(a: string) -> string {\n    let b = a.clone()\n    b\n}\n"));
    }

    #[test]
    fn a_clone_of_a_value_used_later_is_not() {
        assert!(at("fn f(a: string) -> int {\n    let b = a.clone()\n    take(b) + a.len()\n}\n"));
        assert!(at("fn f(a: string) -> int {\n    let b = a.clone()\n    take(b) + take2(\"${a}\")\n}\n"));
    }

    #[test]
    fn a_clone_in_a_loop_of_a_value_used_in_the_loop_is_not() {
        assert!(at("fn f(a: string) {\n    for k in [1, 2] {\n        take(a.clone())\n    }\n}\n"));
    }

    #[test]
    fn a_value_declared_inside_the_loop_is_new_each_time() {
        assert!(!at("fn f() {\n    for k in [1, 2] {\n        let a = make(k)\n        take(a.clone())\n    }\n}\n"));
    }

    #[test]
    fn receivers_that_are_not_plain_locals_keep_the_warning() {
        assert!(!at("fn f(items: Vec<string>) {\n    let first = items[0].clone()\n    use_it(items)\n}\n"));
        assert!(!at("fn f(s: S) {\n    let n = s.name.clone()\n    use_it(s)\n}\n"));
    }
}
