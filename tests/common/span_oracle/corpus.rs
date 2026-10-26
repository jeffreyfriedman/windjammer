//! Source corpora shared by the span gates: every `.wj` file in the repo, plus deterministic
//! generated sources that stress unicode, tabs, CRLF, comments and string interpolation.

use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.wj` file under the repo (examples, tests, std, benchmarks ...), sorted.
pub fn repo_wj_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&repo_root(), &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name == "target" || name.starts_with('.') || name == "node_modules" {
                continue;
            }
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p);
        }
    }
}

pub fn rel(p: &Path) -> String {
    p.strip_prefix(repo_root()).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// xorshift64*: deterministic, no dependency.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

/// Token-level fragments that always lex (no panics): unicode identifiers, every literal form,
/// every operator, comments, interpolation, multi-byte payloads.
pub const FRAGMENTS: &[&str] = &[
    "fn", "let", "mut", "struct", "if", "else", "match", "self", "Self", "true", "false", "_",
    "x", "foo_bar", "café", "日本語", "ß", "名前", "r#type", "x1", "int", "string",
    "0", "42", "1_000", "0xFF", "0xDEAD_BEEF", "0b1010", "0o755", "3.14", "2.5e-3", "1e10",
    "7u64", "0usize", "0xFFu8", "1ux", "3if", "0i", "9inx", "0xFFi",
    "\"hi\"", "\"a\\nb\"", "\"é日本😀\"", "\"tab\\t\\\"q\\\"\"", "\"${x}\"", "\"a ${x + 1} é ${f(\"y\")} z\"",
    "\"line1\nline2\"", "\"crlf\r\nstr\"", "\"$$ ${\"", "r#\"raw \" #\"#", "r##\"r\"##",
    "'a'", "'é'", "'😀'", "'\\n'", "'\\''", "'static", "'a",
    "@route", "@tokio.main", "@",
    "+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=", "&&", "||", "!", "^", "<<", ">>",
    "=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "->", "<-", "=>",
    "(", ")", "{", "}", "[", "]", ",", ".", "..", "..=", ":", "::", ";", "?", "&", "|", "|>", "|>?",
    "// plain comment", "// é日本😀 comment", "//// four slashes", "/// doc comment", "///doc", "/// 😀 doc",
    "/* block */", "/* multi\nline */", "/* nested /* inner */ outer */", "/*",
    "//", "#", "$", "\\", "1e", "0x", "0b", "99999999999999999999", "\u{feff}", "`", "~",
];

pub const SEPARATORS: &[&str] = &[
    " ", "  ", "\t", " \t ", "\n", "\r\n", "\n\n", "\r\n\r\n", "\u{a0}", "\u{3000}", "\n    ", "\r\n\t",
];

/// A generated source of `n_frags` fragments. Line comments are always followed by a newline
/// separator so they cannot swallow the next fragment (which is fine for correctness but makes
/// less interesting tests).
pub fn generate(rng: &mut Rng, n_frags: usize) -> String {
    let mut s = String::new();
    for _ in 0..n_frags {
        let f = *rng.pick(FRAGMENTS);
        s.push_str(f);
        if f.starts_with("//") {
            s.push_str(*rng.pick(&["\n", "\r\n"]));
        } else if rng.below(4) != 0 {
            s.push_str(rng.pick(SEPARATORS));
        }
    }
    s
}

/// Hand-written sources, one per risk named in the audit (multi-line strings/comments, spans after
/// interpolation, CRLF, tabs, astral chars, trailing comment at EOF, no trailing newline).
pub const HAND_WRITTEN: &[&str] = &[
    "",
    " ",
    "\n",
    "\r\n",
    "x",
    "x // trailing comment at EOF",
    "x // trailing comment with é日本😀",
    "// only a comment",
    "fn main() {\r\n\tlet é = \"日本😀\"\r\n\tprint(é)\r\n}\r\n",
    "let s = \"multi\nline\nstring\"\nlet after = 1",
    "let s = \"multi\r\nline\r\nstring\"\r\nlet after = 1",
    "let s = \"a ${x} b ${y + 1} c\" + after",
    "let s = \"é ${x} 日本 ${ {a: 1} } 😀\" ; after",
    "let n = 0ux + 1\nlet m = 3if + y",
    "let n = 0xFFi + 1\nafter",
    "/// doc\n/// doc2\nfn f() {}",
    "\t\tx\t=\t1\t// c\n\ty",
    "a\u{3000}b\u{a0}c",
    "'😀' x \"😀\" y",
    "r#\"raw\nmultiline\"# after",
    "\u{feff}fn main() {}",
    "let a = 1e\nlet b = 0x\nlet c = 99999999999999999999\nlet d = # $ ` ~\nlet e = 2",
    "x // é\n// 日本 comment\ny",
    "a /* é日本 */ b /* two\r\nlines */ c\nd",
    "x /* unterminated é",
];
