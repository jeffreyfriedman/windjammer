//! `wj run --stdin-text '<text>'`: the text is the program's standard input, interpreted or compiled. One shell-safe
//! line carries multi-line input, so an agent needs no redirect, pipe or extra Write call.
//!
//! The text accepts C-style escapes: `\n` `\t` `\r` `\\` `\'` `\"`, `\xNN` (two hex digits, one byte), `\NNN`
//! (one to three octal digits, one byte, at most `\377`) and `\u{H..}` (a Unicode scalar, written as UTF-8). Any other
//! backslash sequence is an error naming its byte offset. A literal `=`, `$`, `<` or `>` (which some agent permission
//! checks refuse inside a command) can be written `\x3d`, `\x24`, `\x3c`, `\x3e`.
//!
//! Like `--stdin-file`, the option is taken out of the command line and the same command runs again with the bytes as
//! stdin, so both engines see exactly the same input.

use std::io::Write;

/// A bad escape: where (byte offset into the text) and why.
#[derive(Debug, PartialEq, Eq)]
pub struct EscapeError {
    pub offset: usize,
    pub message: String,
}

fn err<T>(offset: usize, message: impl Into<String>) -> Result<T, EscapeError> {
    Err(EscapeError { offset, message: message.into() })
}

/// Decode the escapes of `text` into the bytes of the input.
pub fn unescape(text: &str) -> Result<Vec<u8>, EscapeError> {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        let start = i;
        let Some(&c) = b.get(i + 1) else { return err(start, "a lone `\\` at the end of the text") };
        i += 2;
        match c {
            b'n' => out.push(b'\n'),
            b't' => out.push(b'\t'),
            b'r' => out.push(b'\r'),
            b'\\' | b'\'' | b'"' => out.push(c),
            b'x' => {
                let hex = b.get(i..i + 2).and_then(|h| std::str::from_utf8(h).ok());
                match hex.and_then(|h| if h.bytes().all(|d| d.is_ascii_hexdigit()) { u8::from_str_radix(h, 16).ok() } else { None }) {
                    Some(v) => out.push(v),
                    None => return err(start, "`\\x` needs exactly two hex digits (for example `\\x3d`)"),
                }
                i += 2;
            }
            b'0'..=b'7' => {
                let mut v = u32::from(c - b'0');
                let mut n = 1;
                while n < 3 && matches!(b.get(i), Some(b'0'..=b'7')) {
                    v = v * 8 + u32::from(b[i] - b'0');
                    i += 1;
                    n += 1;
                }
                if v > 255 {
                    return err(start, "octal escape above `\\377`");
                }
                out.push(v as u8);
            }
            b'u' => {
                if b.get(i) != Some(&b'{') {
                    return err(start, "`\\u` needs braces: `\\u{1F600}`");
                }
                let Some(close) = b[i..].iter().position(|&d| d == b'}') else { return err(start, "`\\u{` is not closed by `}`") };
                let hex = &text[i + 1..i + close];
                if hex.is_empty() || hex.len() > 6 || !hex.bytes().all(|d| d.is_ascii_hexdigit()) {
                    return err(start, "`\\u{...}` holds one to six hex digits");
                }
                let Some(ch) = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) else {
                    return err(start, "`\\u{...}` is not a Unicode scalar value");
                };
                let mut buf = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                i += close + 1;
            }
            _ => {
                // report the whole (possibly multi-byte) character
                let ch = text[start + 1..].chars().next().unwrap_or('?');
                i = start + 1 + ch.len_utf8();
                return err(start, format!("unknown escape `\\{ch}` (use \\n \\t \\r \\\\ \\' \\\" \\xNN \\NNN \\u{{...}})"));
            }
        }
    }
    Ok(out)
}

/// The value after the first `FLAG` before a `--`, and the command line without both.
fn take(args: &[String], flag: &str) -> Option<(String, Vec<String>)> {
    let i = args.iter().position(|a| a == flag)?;
    let end = args.iter().position(|a| a == "--").unwrap_or(args.len());
    if i >= end {
        return None;
    }
    let value = args.get(i + 1)?.clone();
    let mut rest = args.to_vec();
    rest.drain(i..=i + 1);
    Some((value, rest))
}

/// `--stdin-text` on the command line: the exit status of the re-run (or of the refusal); `None` when absent.
pub fn intercept(args: &[String]) -> Option<i32> {
    let (text, rest) = take(args, "--stdin-text")?;
    if take(args, "--stdin-file").is_some() {
        eprintln!("error: --stdin-text and --stdin-file both give standard input: use one of them");
        return Some(2);
    }
    let bytes = match unescape(&text) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: --stdin-text: {} (at byte {} of the text)", e.message, e.offset);
            return Some(2);
        }
    };
    Some(run_with_bytes(bytes, &rest))
}

fn run_with_bytes(bytes: Vec<u8>, rest: &[String]) -> i32 {
    let Ok(exe) = std::env::current_exe() else { return 1 };
    let spawned = std::process::Command::new(exe).args(rest).stdin(std::process::Stdio::piped()).spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot run wj: {e}");
            return 1;
        }
    };
    let feeder = child.stdin.take().map(|mut s| std::thread::spawn(move || {
        let _ = s.write_all(&bytes); // a program that stops reading early closes the pipe: not an error
    }));
    let status = child.wait();
    if let Some(f) = feeder {
        let _ = f.join();
    }
    status.map(|s| s.code().unwrap_or(1)).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Vec<u8> {
        unescape(s).unwrap()
    }

    #[test]
    fn plain_and_empty() {
        assert_eq!(ok(""), b"");
        assert_eq!(ok("abc def"), b"abc def");
    }

    #[test]
    fn simple_escapes() {
        assert_eq!(ok(r"a\nb\tc\rd"), b"a\nb\tc\rd");
        assert_eq!(ok(r#"\\ \' \""#), b"\\ ' \"");
    }

    #[test]
    fn hex_escapes() {
        assert_eq!(ok(r"\x3d\x24\x3c\x3E"), b"=$<>");
        assert_eq!(ok(r"\xff"), vec![0xff]);
        assert_eq!(ok(r"\x41B"), b"AB");
    }

    #[test]
    fn octal_escapes() {
        assert_eq!(ok(r"\0"), vec![0]);
        assert_eq!(ok(r"\75"), b"=");
        assert_eq!(ok(r"\075"), b"=");
        assert_eq!(ok(r"\377"), vec![255]);
        assert_eq!(ok(r"\1011"), b"A1", "at most three digits");
    }

    #[test]
    fn unicode_escapes_and_raw_utf8() {
        assert_eq!(ok(r"\u{3d}"), b"=");
        assert_eq!(ok(r"\u{e9}"), "é".as_bytes());
        assert_eq!(ok(r"\u{1F600}"), "😀".as_bytes());
        assert_eq!(ok("héllo ✓"), "héllo ✓".as_bytes());
    }

    #[test]
    fn invalid_escapes_name_the_offset() {
        assert_eq!(unescape(r"ab\q").unwrap_err().offset, 2);
        assert_eq!(unescape(r"a\").unwrap_err().offset, 1);
        assert_eq!(unescape(r"\x4").unwrap_err().offset, 0);
        assert_eq!(unescape(r"\xzz").unwrap_err().offset, 0);
        assert_eq!(unescape(r"\400").unwrap_err().offset, 0);
        assert_eq!(unescape(r"\u41").unwrap_err().offset, 0);
        assert_eq!(unescape(r"x\u{41").unwrap_err().offset, 1);
        assert_eq!(unescape(r"\u{}").unwrap_err().offset, 0);
        assert_eq!(unescape(r"\u{D800}").unwrap_err().offset, 0);
        assert_eq!(unescape(r"\u{1234567}").unwrap_err().offset, 0);
        assert_eq!(unescape(r"é\é").unwrap_err().offset, 2, "multi-byte character after the backslash");
    }

    #[test]
    fn option_is_taken_out_and_conflicts_are_refused() {
        let v = |s: &str| s.split(' ').map(String::from).collect::<Vec<_>>();
        assert_eq!(take(&v("run --stdin-text a m.wj"), "--stdin-text"), Some(("a".into(), v("run m.wj"))));
        assert_eq!(take(&v("run m.wj -- --stdin-text a"), "--stdin-text"), None);
        assert_eq!(intercept(&v("run --stdin-text a --stdin-file f m.wj")), Some(2));
        assert_eq!(intercept(&v("run --stdin-text a\\q m.wj")), Some(2));
        assert_eq!(intercept(&v("run m.wj")), None);
    }
}
