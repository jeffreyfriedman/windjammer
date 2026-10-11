//! Per-file semantic checks over the AST: undefined values (WJ0001), unknown structs and
//! fields (WJ0005, WJ0009, WJ0014), literal type mismatches (WJ0003) and duplicate top-level
//! definitions (WJ0021). Conservative by construction: every check must be sound on programs
//! that build, which `tests/check_corpus_test.rs` verifies over the repository corpus.

use super::index::{is_std_name, FileNames, Index};
use super::mismatch::check_literal;
use super::visit::{walk_block, walk_expr, walk_pattern, Visitor};
use crate::check::locate::Locator;
use crate::diagnostics::{Applicability, Diagnostic, Edit, Fix, SourceIndex, Span};
use crate::fuzzy_matcher::levenshtein_distance;
use crate::parser::{Expression, Item, Parameter, Pattern, SourceLocation, Statement, Type};
use std::collections::{HashMap, HashSet};

pub struct FileInput<'a> {
    pub file: &'a str,
    pub text: &'a str,
    pub items: &'a [Item<'static>],
}

pub fn check_file(idx: &Index, input: &FileInput<'_>, out: &mut Vec<Diagnostic>) {
    let six = SourceIndex::new(input.file, input.text);
    let names = FileNames::collect(input.items);
    let mut cx = Cx {
        idx,
        names: &names,
        loc: Locator::new(&six),
        out,
    };
    cx.items(input.items, true);
}

struct Cx<'a, 'o> {
    idx: &'a Index,
    names: &'a FileNames,
    loc: Locator<'a>,
    out: &'o mut Vec<Diagnostic>,
}

fn type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Custom(n) => Some(n.as_str()),
        Type::Reference(t) | Type::MutableReference(t) => type_name(t),
        _ => None,
    }
}

impl Cx<'_, '_> {
    fn items(&mut self, items: &[Item<'static>], top_level: bool) {
        let mut seen_values: HashMap<String, usize> = HashMap::new();
        let mut seen_types: HashMap<String, usize> = HashMap::new();
        for item in items {
            match item {
                Item::Function { decl, .. } => {
                    if top_level {
                        self.duplicate("fn", &decl.name, &mut seen_values);
                    }
                    self.body(&decl.parameters, &decl.body, None);
                }
                Item::Impl { block, .. } => {
                    let self_ty = block.type_name.split('<').next().unwrap_or("").to_string();
                    for f in &block.functions {
                        self.body(&f.parameters, &f.body, Some(&self_ty));
                    }
                }
                Item::Trait { decl, .. } => {
                    if top_level {
                        self.duplicate("trait", &decl.name, &mut seen_types);
                    }
                    for m in &decl.methods {
                        if let Some(body) = &m.body {
                            self.body(&m.parameters, body, None);
                        }
                    }
                }
                Item::Struct { decl, .. } => {
                    if top_level {
                        self.duplicate("struct", &decl.name, &mut seen_types);
                    }
                }
                Item::Enum { decl, .. } => {
                    if top_level {
                        self.duplicate("enum", &decl.name, &mut seen_types);
                    }
                }
                Item::Const { name, value, .. } | Item::Static { name, value, .. } => {
                    if top_level {
                        let kw = if matches!(item, Item::Const { .. }) { "const" } else { "static" };
                        self.duplicate(kw, name, &mut seen_values);
                    }
                    self.free_expr(value);
                }
                Item::Mod { items, .. } => self.items(items, false),
                _ => {}
            }
        }
    }

    fn duplicate(&mut self, kw: &str, name: &str, seen: &mut HashMap<String, usize>) {
        let n = seen.entry(name.to_string()).or_insert(0);
        let nth = *n;
        *n += 1;
        if nth == 0 {
            return;
        }
        let mut d = Diagnostic::error(
            "WJ0021",
            format!("the name `{name}` is defined more than once in this module"),
        );
        if let Some(sp) = self.loc.top_level_decl(kw, name, nth) {
            d = d.at(sp.with_label("redefined here"));
        }
        if let Some(first) = self.loc.top_level_decl(kw, name, 0) {
            d = d.with_secondary(first.with_label("first defined here"));
        }
        self.out.push(d.with_help("rename one of them or delete the duplicate"));
    }

    fn free_expr(&mut self, e: &Expression<'_>) {
        let empty = HashSet::new();
        let mut ck = Checker::new(self, &empty, HashMap::new(), None);
        walk_expr(&mut ck, e);
    }

    fn body(&mut self, params: &[Parameter<'_>], body: &[&Statement<'_>], self_ty: Option<&str>) {
        let mut col = Collect::default();
        let mut typed: HashMap<String, String> = HashMap::new();
        for p in params {
            col.bind(&p.name);
            if let Some(pat) = &p.pattern {
                walk_pattern(&mut col, pat);
            }
            if let Some(t) = type_name(&p.type_) {
                typed.insert(p.name.clone(), t.to_string());
            }
        }
        walk_block(&mut col, body);
        let Collect { names, counts, let_types } = col;
        typed.retain(|n, _| counts.get(n).copied().unwrap_or(0) <= 1);
        for (n, t) in let_types {
            if counts.get(&n).copied().unwrap_or(0) == 1 {
                typed.insert(n, t);
            }
        }
        let mut ck = Checker::new(self, &names, typed, self_ty.map(str::to_string));
        walk_block(&mut ck, body);
    }
}

/// First pass over a function: every name bound anywhere in it, how often, and the struct type
/// of `let x = Point { .. }` / `let x: Point` bindings.
#[derive(Default)]
struct Collect {
    names: HashSet<String>,
    counts: HashMap<String, usize>,
    let_types: HashMap<String, String>,
}

impl Collect {
    fn bind(&mut self, n: &str) {
        self.names.insert(n.to_string());
        *self.counts.entry(n.to_string()).or_default() += 1;
    }
}

impl Visitor for Collect {
    fn pattern(&mut self, p: &Pattern<'_>) {
        match p {
            Pattern::Identifier(n) | Pattern::Ref(n) | Pattern::RefMut(n) | Pattern::MutBinding(n) => {
                self.bind(n)
            }
            Pattern::EnumVariant(_, crate::parser::EnumPatternBinding::Single(n)) => self.bind(n),
            _ => {}
        }
    }
    fn closure(&mut self, params: &[String]) {
        params.iter().for_each(|p| self.bind(p));
    }
    fn stmt(&mut self, s: &Statement<'_>) {
        if let Statement::Let {
            pattern: Pattern::Identifier(n) | Pattern::MutBinding(n),
            type_,
            value,
            ..
        } = s
        {
            let ty = type_.as_ref().and_then(type_name).map(str::to_string).or_else(|| match value {
                Expression::StructLiteral { name, .. } if !name.contains("::") => Some(name.clone()),
                _ => None,
            });
            if let Some(t) = ty {
                self.let_types.insert(n.clone(), t);
            }
        }
    }
}

struct Checker<'c, 'a, 'o> {
    cx: &'c mut Cx<'a, 'o>,
    locals: &'c HashSet<String>,
    typed: HashMap<String, String>,
    self_ty: Option<String>,
    skip: HashSet<usize>,
    macro_locs: Vec<SourceLocation>,
}

impl<'c, 'a, 'o> Checker<'c, 'a, 'o> {
    fn new(
        cx: &'c mut Cx<'a, 'o>,
        locals: &'c HashSet<String>,
        typed: HashMap<String, String>,
        self_ty: Option<String>,
    ) -> Self {
        Self {
            cx,
            locals,
            typed,
            self_ty,
            skip: HashSet::new(),
            macro_locs: Vec::new(),
        }
    }

    /// Location to search from: identifiers inside `${..}` carry the sub-parser's own
    /// coordinates, so they are located relative to the enclosing macro instead.
    fn effective(&self, loc: &SourceLocation) -> SourceLocation {
        match (loc, self.macro_locs.last()) {
            (Some(l), Some(m)) if l.file.as_os_str().is_empty() => m.clone(),
            _ => loc.clone(),
        }
    }

    fn defined(&self, name: &str) -> bool {
        name.is_empty()
            || name.starts_with(|c: char| c.is_uppercase() || c == '_')
            || name.contains("::")
            || self.locals.contains(name)
            || self.cx.names.has_value(name)
            || self.cx.names.glob
            || is_std_name(name)
            || self.cx.idx.fns.contains_key(name)
    }

    fn identifier(&mut self, name: &str, loc: &SourceLocation) {
        if self.defined(name) {
            return;
        }
        let eff = self.effective(loc);
        let mut span = self.cx.loc.word(&eff, name);
        if span.is_none() && !self.macro_locs.is_empty() {
            span = self.cx.loc.first_use(name);
        }
        let mut d = Diagnostic::error("WJ0001", format!("cannot find value `{name}` in this scope"));
        if let Some(sp) = &span {
            d = d.at(sp.clone().with_label("not found in this scope"));
        }
        let candidates = self
            .locals
            .iter()
            .chain(self.cx.names.values.iter())
            .chain(self.cx.names.imported.iter());
        match closest(name, candidates) {
            Some(best) => {
                d = d.with_help(format!("a value with a similar name exists: `{best}`"));
                if let Some(sp) = span {
                    d = d.with_fix(replace_fix(&sp, &best, format!("replace `{name}` with `{best}`")));
                }
            }
            None => {
                d = d.with_help(format!(
                    "declare it before use, for example `let {name} = ...`, or check the spelling"
                ));
            }
        }
        self.cx.out.push(d);
    }

    fn known_struct_fields(&self, ty: &str) -> Option<&'a [(String, Type)]> {
        let idx: &'a Index = self.cx.idx;
        match idx.structs.get(ty).map(Vec::as_slice) {
            Some([one]) if !one.generic && !one.is_tuple => Some(one.fields.as_slice()),
            _ => None,
        }
    }

    fn struct_literal(&mut self, name: &str, fields: &[(String, &Expression<'_>)], loc: &SourceLocation) {
        if name.contains("::") || name == "Self" {
            return;
        }
        let idx: &'a Index = self.cx.idx;
        let Some(infos) = idx.structs.get(name) else {
            self.unknown_struct(name, loc);
            return;
        };
        let Some(decl_fields) = self.known_struct_fields(name) else {
            let _ = infos;
            return;
        };
        let declared: Vec<&str> = decl_fields.iter().map(|(n, _)| n.as_str()).collect();
        let mut unknown = false;
        for (fname, value) in fields {
            if declared.contains(&fname.as_str()) {
                if let (Expression::Literal { value: lit, location }, Some((_, ty))) =
                    (value, decl_fields.iter().find(|(n, _)| n == fname))
                {
                    let span = self.cx.loc.literal(location, lit);
                    let ctx = format!(" in field `{fname}` of `{name}`");
                    if let Some(d) = check_literal(lit, ty, self.cx.idx, span, &ctx) {
                        self.cx.out.push(d);
                    }
                }
                continue;
            }
            unknown = true;
            let span = self.cx.loc.word(loc, fname);
            let mut d = Diagnostic::error(
                "WJ0014",
                format!("struct `{name}` has no field named `{fname}`"),
            );
            if let Some(sp) = &span {
                d = d.at(sp.clone().with_label("unknown field"));
            }
            d = d.with_help(format!("`{name}` has the fields: {}", declared.join(", ")));
            if let (Some(best), Some(sp)) = (closest(fname, declared.iter().map(|s| s.to_string()).collect::<Vec<_>>().iter()), span) {
                d = d.with_fix(replace_fix(&sp, &best, format!("rename the field to `{best}`")));
            }
            self.cx.out.push(d);
        }
        if unknown {
            return;
        }
        let given: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
        let missing: Vec<&str> = declared.iter().copied().filter(|d| !given.contains(d)).collect();
        if !missing.is_empty() {
            let list = missing.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(", ");
            let mut d = Diagnostic::error(
                "WJ0009",
                format!("missing field{} {list} in initializer of `{name}`", if missing.len() > 1 { "s" } else { "" }),
            );
            if let Some(sp) = self.cx.loc.word(loc, name) {
                d = d.at(sp.with_label(format!("missing {list}")));
            }
            self.cx.out.push(d.with_help(format!("add the missing field{}", if missing.len() > 1 { "s" } else { "" })));
        }
    }

    fn unknown_struct(&mut self, name: &str, loc: &SourceLocation) {
        let known = self.cx.idx.types.contains(name)
            || self.cx.names.has_value(name)
            || self.cx.names.glob
            || is_std_name(name)
            || crate::type_classification::is_known_copy_aggregate(name);
        if known || PRELUDE_TYPES.contains(&name) {
            return;
        }
        let mut d = Diagnostic::error("WJ0005", format!("cannot find struct `{name}` in this scope"));
        if let Some(sp) = self.cx.loc.word(loc, name) {
            d = d.at(sp.with_label("not found in this scope"));
        }
        let structs: Vec<String> = self.cx.idx.structs.keys().cloned().collect();
        match closest(name, structs.iter()) {
            Some(best) => d = d.with_help(format!("a struct with a similar name exists: `{best}`")),
            None => d = d.with_help(format!("define it with `struct {name} {{ .. }}` or import it with `use`")),
        }
        self.cx.out.push(d);
    }

    fn field_access(&mut self, object: &Expression<'_>, field: &str, loc: &SourceLocation) {
        let Expression::Identifier { name, .. } = object else { return };
        let ty = if name == "self" {
            self.self_ty.clone()
        } else {
            self.typed.get(name).cloned()
        };
        let Some(ty) = ty else { return };
        if field.chars().all(|c| c.is_ascii_digit()) {
            return;
        }
        let Some(decl_fields) = self.known_struct_fields(&ty) else { return };
        if decl_fields.iter().any(|(n, _)| n == field) {
            return;
        }
        let declared: Vec<String> = decl_fields.iter().map(|(n, _)| n.clone()).collect();
        let mut span = self.cx.loc.word(loc, field);
        if span.is_none() && !self.macro_locs.is_empty() {
            span = self.cx.loc.first_use(field);
        }
        let mut d = Diagnostic::error("WJ0014", format!("no field `{field}` on type `{ty}`"));
        if let Some(sp) = &span {
            d = d.at(sp.clone().with_label("unknown field"));
        }
        d = d.with_help(format!("`{ty}` has the fields: {}", declared.join(", ")));
        if let (Some(best), Some(sp)) = (closest(field, declared.iter()), span) {
            d = d.with_fix(replace_fix(&sp, &best, format!("rename the field to `{best}`")));
        }
        self.cx.out.push(d);
    }

    fn call_literals(&mut self, name: &str, arguments: &[(Option<String>, &Expression<'_>)]) {
        if name.contains("::") || self.locals.contains(name) {
            return;
        }
        let idx: &'a Index = self.cx.idx;
        let Some([info]) = idx.fns.get(name).map(Vec::as_slice) else { return };
        if info.generic || info.params.len() != arguments.len() || arguments.iter().any(|(l, _)| l.is_some()) {
            return;
        }
        for ((_, arg), ty) in arguments.iter().zip(&info.params) {
            if let Expression::Literal { value, location } = arg {
                let span = self.cx.loc.literal(location, value);
                let ctx = format!(" in argument of `{name}`");
                if let Some(d) = check_literal(value, ty, idx, span, &ctx) {
                    self.cx.out.push(d);
                }
            }
        }
    }
}

const PRELUDE_TYPES: &[&str] = &[
    "Vec", "String", "Option", "Result", "Box", "HashMap", "HashSet", "BTreeMap", "BTreeSet",
    "VecDeque", "Rc", "Arc", "RefCell", "Mutex", "Duration", "Instant",
];

impl Visitor for Checker<'_, '_, '_> {
    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Call { function, arguments, .. } => {
                self.skip.insert(*function as *const Expression<'_> as usize);
                if let Expression::Identifier { name, .. } = function {
                    self.call_literals(name, arguments);
                }
            }
            Expression::Identifier { name, location } => {
                if !self.skip.contains(&(e as *const Expression<'_> as usize)) {
                    self.identifier(name, location);
                }
            }
            Expression::StructLiteral { name, fields, location } => {
                let loc = self.effective(location);
                self.struct_literal(name, fields, &loc);
            }
            Expression::FieldAccess { object, field, location } => {
                let loc = self.effective(location);
                self.field_access(object, field, &loc);
            }
            _ => {}
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        if let Statement::Let {
            type_: Some(ty),
            value: Expression::Literal { value, location },
            ..
        } = s
        {
            let span = self.cx.loc.literal(location, value);
            if let Some(d) = check_literal(value, ty, self.cx.idx, span, "") {
                self.cx.out.push(d);
            }
        }
    }

    fn macro_enter(&mut self, e: &Expression<'_>) {
        self.macro_locs.push(e.location());
    }
    fn macro_exit(&mut self) {
        self.macro_locs.pop();
    }
}

/// Closest candidate by edit distance (at most a third of the name's length, at least one edit).
fn closest<'s>(name: &str, candidates: impl Iterator<Item = &'s String>) -> Option<String> {
    let max = (name.chars().count() / 3).clamp(1, 3);
    candidates
        .filter(|c| c.as_str() != name)
        .map(|c| (levenshtein_distance(name, c), c))
        .filter(|(d, _)| *d <= max)
        .min_by_key(|(d, c)| (*d, (*c).clone()))
        .map(|(_, c)| c.clone())
}

fn replace_fix(span: &Span, replacement: &str, description: String) -> Fix {
    Fix {
        description,
        applicability: Applicability::MaybeIncorrect,
        edits: vec![Edit {
            span: Span { label: None, ..span.clone() },
            replacement: replacement.to_string(),
        }],
    }
}
