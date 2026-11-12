//! `WJ0003` for a `match` whose arm patterns name a variant of a different enum than the scrutinee's known type:
//! `Some(v)` / `None` against a `Result`, `Ok(v)` / `Err(e)` against an `Option`. The compiled build rejects it and
//! the interpreter panics at run time ("no match arm matched value of type Result").
//!
//! The scrutinee's type is known only from a declaration, never guessed from a name: a call to a project function
//! with a declared `Result` / `Option` return, a std function in `stdlib_surface::std_functions::returns`, or a
//! local bound once from such a call or declared with such a type. Anything else is not judged.

use super::front::Parsed;
use super::locate::{masked_ranges, Locator};
use super::semantic::{walk_block, Index, Visitor};
use super::std_calls::std_imports;
use crate::diagnostics::{Applicability, Diagnostic, Edit, Fix, SourceIndex, Span};
use crate::parser::ast::core::{EnumPatternBinding, Expression, FunctionDecl, Item, MatchArm, Pattern, Statement, Type};
use crate::stdlib_surface::std_functions::{self, Wrapper};
use std::collections::{HashMap, HashSet};

pub fn run(parsed: &[&Parsed], idx: &Index, out: &mut Vec<Diagnostic>) {
    // a project enum with a variant of the same name makes the pattern ambiguous: not judged
    let user_variants: HashSet<&str> = idx.variants.values().flatten().map(String::as_str).collect();
    for p in parsed {
        let six = SourceIndex::new(p.file.clone(), &p.text);
        let mut w = Matches {
            idx,
            aliases: std_imports(&p.program.items),
            user_variants: &user_variants,
            six: &six,
            loc: Locator::new(&six),
            masked: masked_ranges(&p.text),
            known: HashMap::new(),
            bound: HashMap::new(),
            out: &mut *out,
        };
        w.items(&p.program.items);
    }
}

struct Matches<'a, 'o> {
    idx: &'a Index,
    aliases: HashMap<String, String>,
    user_variants: &'a HashSet<&'a str>,
    six: &'a SourceIndex<'a>,
    loc: Locator<'a>,
    masked: Vec<(usize, usize)>,
    /// Locals of the current function bound exactly once, with the wrapper their declaration gives them.
    known: HashMap<String, Wrapper>,
    /// How often each name is bound in the current function (parameters, patterns, closure parameters).
    bound: HashMap<String, usize>,
    out: &'o mut Vec<Diagnostic>,
}

/// Counts bindings so a name that is rebound or shadowed is never judged.
struct Binds<'b>(&'b mut HashMap<String, usize>);

impl Visitor for Binds<'_> {
    fn pattern(&mut self, p: &Pattern<'_>) {
        match p {
            Pattern::Identifier(n) | Pattern::Ref(n) | Pattern::RefMut(n) | Pattern::MutBinding(n) => *self.0.entry(n.clone()).or_default() += 1,
            Pattern::EnumVariant(_, EnumPatternBinding::Single(n)) => *self.0.entry(n.clone()).or_default() += 1,
            _ => {}
        }
    }
    fn closure(&mut self, params: &[String]) {
        for p in params {
            p.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).for_each(|w| *self.0.entry(w.to_string()).or_default() += 1);
        }
    }
}

fn wrapper_of_type(t: &Type) -> Option<Wrapper> {
    match t {
        Type::Option(_) => Some(Wrapper::Option),
        Type::Result(..) => Some(Wrapper::Result),
        Type::Reference(t) | Type::MutableReference(t) => wrapper_of_type(t),
        _ => None,
    }
}

/// `Some`/`None`/`Ok`/`Err` as spelled in a pattern path (`Option::Some` too); other paths are some other enum's.
fn variant_of(path: &str) -> Option<&'static str> {
    let segs: Vec<&str> = path.split("::").collect();
    let v = match *segs.last()? {
        "Some" => "Some",
        "None" => "None",
        "Ok" => "Ok",
        "Err" => "Err",
        _ => return None,
    };
    match segs.len() {
        1 => Some(v),
        n if matches!(segs[n - 2], "Option" | "Result") => Some(v),
        _ => None,
    }
}

fn wrapper_of_variant(v: &str) -> Wrapper {
    if matches!(v, "Some" | "None") {
        Wrapper::Option
    } else {
        Wrapper::Result
    }
}

/// The wrapper's variant that takes the place of `v`, and whether the replacement keeps the arm's meaning.
fn counterpart(v: &str) -> &'static str {
    match v {
        "Some" => "Ok",
        "None" => "Err(_)",
        "Ok" => "Some",
        _ => "None",
    }
}

/// An arm pattern's variants that belong to the other wrapper, with whether the arm binds a value.
fn foreign(p: &Pattern<'_>, scrutinee: Wrapper, out: &mut Vec<(&'static str, bool)>) {
    match p {
        Pattern::Or(ps) => ps.iter().for_each(|q| foreign(q, scrutinee, out)),
        Pattern::EnumVariant(path, b) => {
            if let Some(v) = variant_of(path) {
                if wrapper_of_variant(v) != scrutinee {
                    out.push((v, matches!(b, EnumPatternBinding::Single(_) | EnumPatternBinding::Tuple(_))));
                }
            }
        }
        Pattern::Identifier(n) => {
            if let Some(v) = variant_of(n) {
                if wrapper_of_variant(v) != scrutinee {
                    out.push((v, false));
                }
            }
        }
        _ => {}
    }
}

/// Byte ranges `(word start, word end, pattern end)` of the arm patterns of the `match` at `from` that are spelled
/// with one of `names`, in source order. An arm pattern sits directly inside the match braces (not in a nested
/// bracket) and is followed by `=>`, `|` or a guard; anything else (a `Some(1)` in an arm body) is skipped.
fn arm_patterns(text: &str, masked: &[(usize, usize)], from: usize, names: &[&str]) -> Vec<(usize, usize, usize)> {
    let b = text.as_bytes();
    let is_masked = |at: usize| masked.iter().any(|&(s, e)| at >= s && at < e);
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = from;
    while i < b.len() && !(b[i] == b'{' && !is_masked(i)) {
        i += 1;
    }
    let (mut depth, mut paren) = (0i32, 0i32);
    let mut found = Vec::new();
    while i < b.len() {
        if is_masked(i) {
            i += 1;
            continue;
        }
        match b[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth <= 0 {
                    break;
                }
            }
            b'(' | b'[' => paren += 1,
            b')' | b']' => paren -= 1,
            c if is_word(c) => {
                let start = i;
                while i < b.len() && is_word(b[i]) {
                    i += 1;
                }
                if depth == 1 && paren == 0 && names.contains(&&text[start..i]) {
                    let mut k = i;
                    while k < b.len() && b[k] == b' ' {
                        k += 1;
                    }
                    if k < b.len() && b[k] == b'(' {
                        let mut d = 0;
                        while k < b.len() {
                            match b[k] {
                                b'(' => d += 1,
                                b')' => {
                                    d -= 1;
                                    if d == 0 {
                                        k += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            k += 1;
                        }
                    }
                    let pattern_end = k;
                    while k < b.len() && b[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if text[k..].starts_with("=>") || text[k..].starts_with('|') || text[k..].starts_with("if ") {
                        found.push((start, i, pattern_end));
                    }
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    found
}

impl Matches<'_, '_> {
    fn items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.function(decl),
                Item::Impl { block, .. } => block.functions.iter().for_each(|f| self.function(f)),
                Item::Trait { decl, .. } => {
                    for m in decl.methods.iter().filter(|m| m.body.is_some()) {
                        self.function(m)
                    }
                }
                Item::Mod { items, .. } => self.items(items),
                _ => {}
            }
        }
    }

    fn function(&mut self, f: &FunctionDecl<'_>) {
        self.bound.clear();
        self.known.clear();
        for p in &f.parameters {
            *self.bound.entry(p.name.clone()).or_default() += 1;
        }
        walk_block(&mut Binds(&mut self.bound), &f.body);
        for p in &f.parameters {
            if let Some(w) = wrapper_of_type(&p.type_) {
                if self.bound.get(&p.name) == Some(&1) {
                    self.known.insert(p.name.clone(), w);
                }
            }
        }
        walk_block(self, &f.body);
    }

    /// The wrapper `e` is declared to have, with what produces it (for the message).
    fn wrapper_of(&self, e: &Expression<'_>) -> Option<(Wrapper, String)> {
        match e {
            Expression::Identifier { name, .. } => self.known.get(name).map(|w| (*w, format!("`{name}`"))),
            Expression::Call { function: Expression::Identifier { name, .. }, .. } => {
                if let Some((head, func)) = name.rsplit_once("::") {
                    let module = match head.strip_prefix("std::") {
                        Some(m) if !m.contains("::") => m.to_string(),
                        _ if self.bound.contains_key(head) => return None,
                        _ => self.aliases.get(head)?.clone(),
                    };
                    return std_functions::returns(&module, func).map(|w| (w, format!("`{head}::{func}`")));
                }
                if self.bound.contains_key(name) {
                    return None;
                }
                let Some([info]) = self.idx.fns.get(name).map(Vec::as_slice) else { return None };
                if info.generic {
                    return None;
                }
                info.ret.as_ref().and_then(wrapper_of_type).map(|w| (w, format!("`{name}`")))
            }
            _ => None,
        }
    }

    fn mismatches(&mut self, value: &Expression<'_>, arms: &[MatchArm<'_>], location: &crate::parser::SourceLocation) {
        let Some((scrutinee, what)) = self.wrapper_of(value) else { return };
        let mut wrong = Vec::new();
        for arm in arms {
            foreign(&arm.pattern, scrutinee, &mut wrong);
        }
        wrong.retain(|(v, _)| !self.user_variants.contains(v));
        if wrong.is_empty() {
            return;
        }
        let (expected, found) = match scrutinee {
            Wrapper::Result => ("Result", "Option"),
            Wrapper::Option => ("Option", "Result"),
        };
        let match_span = self.loc.word(location, "match");
        let from = match_span.as_ref().map_or(0, |s| s.byte_start);
        let names: Vec<&str> = wrong.iter().map(|(v, _)| *v).collect();
        let text = self.six.text();
        let spans = arm_patterns(text, &self.masked, from, &names);
        // positions are trusted only when they pair up one to one with the arms of the AST
        let paired = spans.len() == wrong.len() && spans.iter().zip(&names).all(|(s, n)| &text[s.0..s.1] == *n);
        for (k, (v, binds)) in wrong.iter().enumerate() {
            let shown = if matches!(*v, "None") { "None".to_string() } else { format!("{v}(..)") };
            let mut d = Diagnostic::error("WJ0003", format!("mismatched types: expected `{expected}`, found `{found}` pattern `{shown}`"));
            let fixed = counterpart(v);
            let (ok, err) = if scrutinee == Wrapper::Result { ("Ok(v)", "Err(_)") } else { ("Some(v)", "None") };
            if paired {
                let (start, end, pattern_end) = spans[k];
                let span: Span = self.six.span(start, end);
                d = d.at(span.clone().with_label(format!("an `{found}` pattern, but the value is a `{expected}`")));
                // `Err(e)` -> `None` drops the binding; every other rewrite keeps the arm's meaning
                let (edit_span, replacement, applicability) = if *v == "Err" {
                    (self.six.span(start, pattern_end), fixed.to_string(), if *binds { Applicability::MaybeIncorrect } else { Applicability::MachineApplicable })
                } else {
                    (span, fixed.to_string(), Applicability::MachineApplicable)
                };
                d = d.with_fix(Fix {
                    description: format!("match the `{expected}` with `{fixed}`"),
                    applicability,
                    edits: vec![Edit { span: Span { label: None, ..edit_span }, replacement }],
                });
            } else if let Some(sp) = &match_span {
                d = d.at(sp.clone().with_label(format!("the value matched here is a `{expected}`")));
            }
            d = d.with_help(format!("{what} returns a `{expected}`: match `{ok}` and `{err}` instead of the `{found}` variants (`{v}` becomes `{fixed}`)"));
            self.out.push(d);
        }
    }
}

impl Visitor for Matches<'_, '_> {
    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), type_, value, .. } if self.bound.get(n) == Some(&1) => {
                let w = match type_ {
                    Some(t) => wrapper_of_type(t),
                    None => self.wrapper_of(value).map(|(w, _)| w),
                };
                if let Some(w) = w {
                    self.known.insert(n.clone(), w);
                }
            }
            Statement::Match { value, arms, location } => self.mismatches(value, arms, location),
            _ => {}
        }
    }
}
