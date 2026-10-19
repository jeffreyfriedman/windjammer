//! What the anonymizer may rename. Language and standard-library names stay; everything else is
//! the user's. Protection is decided per *occurrence*, because the same word is often both: a local
//! `next` is the user's, `it.next()` is the standard library's.

use crate::formatter::{scan_lossless, Tok, TokKind};
use std::collections::HashSet;

/// Always the language's: primitive types, prelude types and constructors, macros, derive traits.
const HARD: &str = "_ self Self super crate std main lib mod int uint float bool str char string i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64";

/// Prelude names a program may shadow with its own declaration.
const PRELUDE: &str = "String Vec Option Result Some None Ok Err Box Rc Arc RefCell Cell Mutex RwLock HashMap HashSet BTreeMap BTreeSet VecDeque BinaryHeap Cow \
Debug Clone Copy PartialEq Eq PartialOrd Ord Hash Default Display Send Sync Sized Drop Iterator IntoIterator From Into TryFrom TryInto AsRef AsMut Deref DerefMut Fn FnMut FnOnce \
Add Sub Mul Div Rem Neg Not Index IndexMut Path PathBuf Duration Instant Ordering \
derive test bench inline cfg allow deny warn serde Serialize Deserialize \
println print eprintln eprint format panic assert assert_eq assert_ne debug_assert unreachable todo unimplemented dbg vec write writeln";

/// Standard-library method and function names reached with `.` or `::` or via `use std::..`.
const METHODS: &str = "new default clone to_string to_owned to_vec to_lowercase to_uppercase as_str as_ref as_mut as_bytes into from iter iter_mut into_iter len is_empty push pop insert remove get get_mut contains contains_key \
keys values entries map filter fold collect sum min max count first last take skip rev zip enumerate chain flat_map filter_map find position any all sort sort_by sort_by_key dedup \
extend append clear truncate drain retain join split trim lines chars bytes parse unwrap expect unwrap_or unwrap_or_else unwrap_or_default ok_or ok_or_else and_then or_else is_some is_none is_ok is_err \
borrow borrow_mut lock read write abs pow sqrt floor ceil round powi powf sin cos min_by max_by cmp partial_cmp eq ne lt le gt ge fmt next peek entry or_insert or_insert_with or_default \
starts_with ends_with replace repeat concat reverse swap split_at windows chunks get_or_insert_with take_while skip_while step_by cloned copied last_mut first_mut as_slice";

/// Method names the language dispatches on (operators, formatting, iteration, comparison): a user
/// `fn eq` or `fn to_string` is part of the protocol, so declaration and calls keep the name.
const PROTOCOL: &str = "to_string fmt eq ne cmp partial_cmp lt le gt ge add sub mul div rem neg not index index_mut next drop clone default from into hash deref deref_mut iter into_iter add_assign sub_assign mul_assign div_assign";

#[derive(Default)]
pub struct Names {
    protocol: HashSet<String>,
    reserved: HashSet<String>,
    prelude: HashSet<String>,
    members: HashSet<String>,
}

/// Names the *project* declares; a declaration beats the standard-library reading of the same word.
#[derive(Default)]
pub struct User {
    pub fns: HashSet<String>,
    /// Top-level (non-method) functions; they never take part in a protocol.
    pub free_fns: HashSet<String>,
    pub fields: HashSet<String>,
    pub types: HashSet<String>,
    /// Methods declared inside `impl <std trait> for X` (`fmt`, `next`, `drop`...): the std's names.
    pub std_trait_fns: HashSet<String>,
    /// Every word of a `use std::...` statement.
    pub std_imports: HashSet<String>,
}

impl Names {
    pub fn new() -> Names {
        let mut n = Names::default();
        n.reserved
            .extend(HARD.split_whitespace().map(str::to_string));
        n.protocol
            .extend(PROTOCOL.split_whitespace().map(str::to_string));
        n.prelude
            .extend(PRELUDE.split_whitespace().map(str::to_string));
        n.reserved.extend(
            crate::lexer::keywords::KEYWORDS
                .iter()
                .map(|k| k.word.to_string()),
        );
        n.members
            .extend(METHODS.split_whitespace().map(str::to_string));
        for (rel, bytes) in crate::embedded_assets::STD_WJ {
            let stem = std::path::Path::new(rel)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            n.members.insert(stem.to_string());
            if let Ok(toks) = scan_lossless(&String::from_utf8_lossy(bytes)) {
                for (i, t) in toks.iter().enumerate() {
                    let declared = i > 0
                        && matches!(
                            toks[i - 1].text.as_str(),
                            "fn" | "struct"
                                | "enum"
                                | "trait"
                                | "type"
                                | "mod"
                                | "const"
                                | "static"
                        );
                    let called = toks.get(i + 1).is_some_and(|x| x.is("("));
                    if t.kind == TokKind::Word && (declared || called) {
                        n.members.insert(t.text.clone());
                    }
                }
            }
        }
        n
    }

    fn std_qualified(&self, user: &User, q: &str) -> bool {
        q == "std"
            || self.reserved.contains(q)
            || (self.prelude.contains(q) && !user.types.contains(q))
            || (user.std_imports.contains(q) && !user.types.contains(q))
    }

    /// Whether the word at `toks[i]` belongs to the language / std (and so must not be renamed).
    ///
    /// A word is the user's when the project declares it *and* the occurrence can refer to the
    /// declaration. After `.` or `::` an undeclared word is a standard-library member; a plain word
    /// is the user's unless it is reserved, imported from `std`, or a call of a std function.
    pub fn protected_at(&self, user: &User, toks: &[Tok], i: usize) -> bool {
        let w = toks[i].text.as_str();
        if w.starts_with(|c: char| c.is_ascii_digit()) || self.reserved.contains(w) {
            return true;
        }
        if prev_is_fn(toks, i) && user.std_trait_fns.contains(w) && !user.fns.contains(w) {
            return true;
        }
        let shadowed = user.fns.contains(w) || user.types.contains(w);
        if self.prelude.contains(w) && !shadowed {
            return true;
        }
        let prev = if i > 0 { toks[i - 1].text.as_str() } else { "" };
        let next = toks.get(i + 1).map(|t| t.text.as_str()).unwrap_or("");
        if self.protocol.contains(w)
            && !user.free_fns.contains(w)
            && matches!(prev, "fn" | "." | "::")
        {
            return true;
        }
        let declared = user.fns.contains(w) || user.fields.contains(w) || user.types.contains(w);
        if user.std_imports.contains(w) && !declared {
            return true;
        }
        match prev {
            "::" => {
                let q = if i >= 2 {
                    toks[i - 2].text.as_str()
                } else {
                    ""
                };
                self.std_qualified(user, q) || !declared
            }
            "." if next == "(" => !user.fns.contains(w),
            "." => !declared,
            _ => next == "(" && self.members.contains(w) && !user.fns.contains(w),
        }
    }
}

fn prev_is_fn(toks: &[Tok], i: usize) -> bool {
    i > 0 && toks[i - 1].text == "fn"
}

impl User {
    pub fn collect(files: &[Vec<Tok>], names: &Names) -> User {
        let mut u = User::default();
        for toks in files {
            collect_std_imports(toks, &mut u);
        }
        for toks in files {
            for (i, t) in toks.iter().enumerate() {
                if t.kind == TokKind::Word
                    && i > 0
                    && matches!(
                        toks[i - 1].text.as_str(),
                        "struct" | "enum" | "trait" | "type"
                    )
                {
                    u.types.insert(t.text.clone());
                }
            }
            collect_fields(toks, &mut u, names);
        }
        u
    }
}

/// `impl Display for X {` / `impl std::fmt::Display for X {` (a std trait before `for`).
fn is_std_trait_impl(toks: &[Tok], open: usize, user: &User, names: &Names) -> bool {
    let mut start = open;
    while start > 0 && !matches!(toks[start - 1].text.as_str(), "}" | "{" | ";") {
        start -= 1;
    }
    let header = &toks[start..open];
    let (Some(imp), Some(f)) = (
        header.iter().position(|t| t.text == "impl"),
        header.iter().position(|t| t.text == "for"),
    ) else {
        return false;
    };
    f > imp
        && header[imp..f].iter().any(|t| {
            t.kind == TokKind::Word
                && (t.text == "std"
                    || names.prelude.contains(&t.text)
                    || user.std_imports.contains(&t.text))
                && !user.types.contains(&t.text)
        })
}

#[derive(Clone, Copy, PartialEq)]
enum Body {
    Struct,
    Enum,
    StdImpl,
    Other,
}

fn header_kind(toks: &[Tok], open: usize) -> Option<Body> {
    let mut j = open;
    while j > 0 {
        j -= 1;
        match toks[j].text.as_str() {
            "struct" => return Some(Body::Struct),
            "enum" => return Some(Body::Enum),
            "}" | "{" | ";" => return None,
            _ => {}
        }
    }
    None
}

fn collect_fields(toks: &[Tok], u: &mut User, names: &Names) {
    let mut stack: Vec<Body> = Vec::new();
    let mut parens: Vec<i32> = Vec::new(); // open `(`/`[` inside each block
    for (i, t) in toks.iter().enumerate() {
        if t.is("{") {
            let kind = header_kind(toks, i).unwrap_or(if stack.last() == Some(&Body::Enum) {
                Body::Struct
            } else if is_std_trait_impl(toks, i, u, names) {
                Body::StdImpl
            } else {
                Body::Other
            });
            stack.push(kind);
            parens.push(0);
        } else if t.is("}") {
            stack.pop();
            parens.pop();
        } else if t.is("(") || t.is("[") {
            if let Some(p) = parens.last_mut() {
                *p += 1;
            }
        } else if t.is(")") || t.is("]") {
            if let Some(p) = parens.last_mut() {
                *p -= 1;
            }
        } else if t.kind == TokKind::Word && i > 0 && toks[i - 1].text == "fn" {
            if stack.last() == Some(&Body::StdImpl) {
                u.std_trait_fns.insert(t.text.clone());
            } else {
                u.fns.insert(t.text.clone());
                if stack.is_empty() {
                    u.free_fns.insert(t.text.clone());
                }
            }
        } else if t.kind == TokKind::Word && i > 0 && parens.last() == Some(&0) {
            match stack.last() {
                Some(Body::Struct) if toks.get(i + 1).is_some_and(|n| n.is(":")) => {
                    u.fields.insert(t.text.clone());
                }
                Some(Body::Enum) if toks[i - 1].is("{") || toks[i - 1].is(",") => {
                    u.types.insert(t.text.clone());
                }
                _ => {}
            }
        }
    }
}

fn collect_std_imports(toks: &[Tok], u: &mut User) {
    let mut i = 0;
    while i < toks.len() {
        if toks[i].text == "use" && toks.get(i + 1).is_some_and(|t| t.text == "std") {
            let mut depth = 0i32;
            let mut j = i + 1;
            while j < toks.len() && (j == i + 1 || depth > 0 || toks[j].nl == 0) {
                if toks[j].is("{") {
                    depth += 1;
                } else if toks[j].is("}") {
                    depth -= 1;
                }
                if toks[j].kind == TokKind::Word {
                    u.std_imports.insert(toks[j].text.clone());
                }
                j += 1;
            }
            i = j;
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reduce::toks::scan;

    fn prot(src: &str, word: &str, nth: usize) -> bool {
        let toks = scan(src).unwrap();
        let user = User::collect(std::slice::from_ref(&toks), &Names::new());
        let idx: Vec<usize> = toks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.text == word)
            .map(|(i, _)| i)
            .collect();
        Names::new().protected_at(&user, &toks, idx[nth])
    }

    #[test]
    fn language_names_are_protected_user_names_are_not() {
        assert!(prot("fn main() {\n}\n", "fn", 0));
        assert!(prot("let v: Vec<int> = []\n", "Vec", 0));
        assert!(!prot("struct InvoiceLedger {\n}\n", "InvoiceLedger", 0));
        assert!(!prot("fn customer_total() {\n}\n", "customer_total", 0));
    }

    #[test]
    fn the_same_word_is_user_or_std_by_position() {
        let src = "fn f(it: Iter) {\n    let next = it.next()\n    next\n}\n";
        assert!(!prot(src, "next", 0), "local variable");
        assert!(prot(src, "next", 1), "std method call");
    }

    #[test]
    fn enum_variants_and_their_fields_are_the_users() {
        let src = "enum Shape {\n    Circle(int),\n    Rect { w: int, h: int },\n}\nfn f() {\n    let s = Shape::Circle(1)\n    let n = s.to_digit(2)\n}\n";
        assert!(!prot(src, "Circle", 1), "variant after ::");
        assert!(!prot(src, "Rect", 0));
        assert!(!prot(src, "w", 0), "field of a struct-like variant");
        assert!(prot(src, "to_digit", 0), "undeclared method is std");
    }

    #[test]
    fn methods_of_std_trait_impls_keep_their_names() {
        let src = "struct P {\n}\nimpl Display for P {\n    fn fmt(self) -> string {\n        \"p\"\n    }\n}\nimpl P {\n    fn area(self) -> int {\n        1\n    }\n}\n";
        assert!(prot(src, "fmt", 0));
        assert!(!prot(src, "area", 0));
    }

    #[test]
    fn protocol_methods_keep_their_names_even_in_inherent_impls() {
        let src = "impl P {\n    fn to_string(self) -> string {\n        \"p\"\n    }\n}\nfn f(p: P) {\n    p.to_string()\n}\n";
        assert!(prot(src, "to_string", 0));
        assert!(prot(src, "to_string", 1));
    }

    #[test]
    fn a_top_level_function_named_like_a_protocol_method_is_the_users() {
        let src = "fn next(s: int) -> int {\n    s\n}\nfn main() {\n    let a = next(1)\n}\n";
        assert!(!prot(src, "next", 0));
        assert!(!prot(src, "next", 1));
    }

    #[test]
    fn user_methods_and_fields_win_over_std_names() {
        let src = "struct S {\n    count: int,\n}\nimpl S {\n    fn len(self) -> int {\n        self.count\n    }\n}\nfn g(s: S, v: Vec<int>) {\n    s.len()\n    v.push(s.count)\n}\n";
        assert!(!prot(src, "count", 1), "field access of a declared field");
        assert!(!prot(src, "len", 1), "call of a user fn");
        assert!(prot(src, "push", 0), "std call");
    }

    #[test]
    fn std_qualified_paths_and_imports_stay() {
        let src = "use std::collections::HashMap\nfn f() {\n    let m = HashMap::new()\n    let v = Vec::new()\n}\nstruct R {\n}\nimpl R {\n    fn new() -> R {\n        R {}\n    }\n}\nfn h() {\n    let r = R::new()\n}\n";
        assert!(prot(src, "collections", 0));
        assert!(prot(src, "new", 0), "HashMap::new");
        assert!(prot(src, "new", 1), "Vec::new");
        assert!(!prot(src, "new", 3), "R::new where R is the user's");
    }
}
