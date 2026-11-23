//! WJ0173: data from an untrusted source (environment, command line, file contents, standard input, an HTTP request)
//! reaches a command, a file path to write or SQL text without a sanitizer (ADR-0009). `wj check`, `wj build`
//! and `wj run` stop on it. See `flow.rs` for the analysis and `catalog.rs` for what counts as source, sink, sanitizer.

mod catalog;
pub mod flow;

use super::front::Parsed;
use super::locate::Locator;
use crate::diagnostics::{Diagnostic, SourceIndex, Span};
use crate::parser::ast::core::Item;
use crate::resolve::{self, DefKind, ParsedFile};
use flow::{Finding, Pragma, Summary};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

/// The registry code.
pub const CODE: &str = "WJ0173";

/// Names of the `.wj` doc-comment annotations (`/// wj-taint: sanitizer`, `/// wj-taint: trusted(<reason>)`).
pub const ANNOTATION: &str = "wj-taint:";

pub use catalog::{SOURCES, STD_SANITIZERS};

/// Cheap, exact test whether a program can have a finding at all. A finding is a `Source` origin reaching a sink call.
/// * Origins arise only from (1) a call of a std source function (`catalog::SOURCES`), (2) a member of an HTTP request value (a
///   parameter whose type is a runtime type of request shape), and (3) the results of the program's own functions, which hold them
///   only because their bodies contain (1) or (2). So a program none of whose functions calls a function named like a source,
///   imports one (under any alias), takes a request-shaped parameter or imports `http` has no origin at all.
/// * A sink is a call of a std function the effect table gives a process or file-write role, or `query`/`execute` of a database
///   connection (`catalog::sink_roles`, `db_method_role`); a program with no call named like one has no sink.
/// Either absent means no finding is possible and the walk (a second resolution of the whole program and the function summaries)
/// is skipped. The names are the last segments of the tables, so any module path, alias or method spelling is covered; the test is a
/// superset (`read` of anything counts), never a subset.
fn may_flow(parsed: &[&Parsed]) -> bool {
    let sinks = catalog::sink_names();
    // sinks first: the cheaper, and most programs that read input never write a path or start a process
    if !parsed.iter().any(|p| items_match(&p.program.items, &sinks, false)) {
        return false;
    }
    let sources: Vec<&str> = catalog::SOURCES.iter().filter_map(|s| s.path.rsplit("::").next()).collect();
    parsed.iter().any(|p| items_match(&p.program.items, &sources, true))
}

/// Does any function of `items` call (or, for sources, import or take a request parameter) one of `names`?
fn items_match(items: &[Item<'_>], names: &[&str], sources: bool) -> bool {
    items.iter().any(|i| match i {
        Item::Use { path, alias, .. } => {
            sources && (path.iter().any(|seg| seg == "http") || path.last().is_some_and(|l| names.contains(&l.as_str())) || alias.as_deref().is_some_and(|a| names.contains(&a)))
        }
        Item::Function { decl, .. } => function_matches(decl, names, sources),
        Item::Impl { block, .. } => block.functions.iter().any(|f| function_matches(f, names, sources)),
        Item::Trait { decl, .. } => decl.methods.iter().any(|m| m.body.as_ref().is_some_and(|b| body_matches(b, names))),
        Item::Mod { items, .. } => items_match(items, names, sources),
        _ => false,
    })
}

fn function_matches(decl: &crate::parser::ast::core::FunctionDecl<'_>, names: &[&str], sources: bool) -> bool {
    (sources && decl.parameters.iter().any(|p| param_is_request(&p.type_))) || body_matches(&decl.body, names)
}

fn param_is_request(t: &crate::parser::ast::types::Type) -> bool {
    use crate::parser::ast::types::Type;
    match t {
        Type::Custom(n) => crate::ir::taint::is_http_request_type(n),
        Type::Reference(i) | Type::MutableReference(i) | Type::Option(i) | Type::Vec(i) => param_is_request(i),
        Type::Parameterized(n, args) => crate::ir::taint::is_http_request_type(n) || args.iter().any(param_is_request),
        _ => false,
    }
}

fn body_matches(body: &[&crate::parser::ast::core::Statement<'_>], names: &[&str]) -> bool {
    use crate::parser::ast::core::Expression;
    let found = std::cell::Cell::new(false);
    flow::scan(body, &mut |_| {}, &mut |e| {
        if found.get() {
            return;
        }
        let hit = match e {
            Expression::Call { function: Expression::Identifier { name, .. }, .. } => names.contains(&name.rsplit("::").next().unwrap_or(name)),
            Expression::MethodCall { method, .. } => names.contains(&method.as_str()),
            _ => false,
        };
        if hit {
            found.set(true);
        }
    });
    found.get()
}

pub fn run(root: &Path, parsed: &[&Parsed], by_file: &HashMap<String, &Parsed>, out: &mut Vec<Diagnostic>) {
    // The annotation check is syntactic and independent of the pre-scan: a malformed `trusted` is an error even in a program
    // where no source can reach a sink.
    for p in parsed {
        malformed_annotations(&p.program.items, p, out);
    }
    let flows = may_flow(parsed);
    crate::debug_env::phase("check-taint-prescan");
    if !flows {
        return;
    }
    let base = super::effects::module_base(root);
    let inputs: Vec<ParsedFile<'_>> = parsed
        .iter()
        .map(|p| {
            let path = Path::new(&p.file);
            ParsedFile::new(resolve::module_path_from_file(&base, path), path.to_path_buf(), &p.program)
        })
        .collect();
    let resolved = resolve::resolve_program(&inputs);
    let prog = flow::Program::new(&resolved);
    let defs = resolved.defs();
    let fns: Vec<_> = resolved
        .collected
        .decls
        .iter()
        .filter(|(id, d)| !d.body.is_empty() && matches!(defs.get(**id).kind, DefKind::Fn | DefKind::Method { .. } | DefKind::AssocFn { .. }))
        .filter_map(|(id, d)| defs.get(*id).location.as_ref().map(|l| (*id, *d, l.file.to_string_lossy().to_string())))
        .collect();
    let mut summaries: HashMap<_, Summary> = HashMap::new();
    for _ in 0..12 {
        let mut changed = false;
        for (id, d, file) in &fns {
            let o = flow::analyze_fn(&prog, &summaries, d, file);
            if summaries.get(id) != Some(&o.summary) {
                summaries.insert(*id, o.summary);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut findings: BTreeSet<Finding> = BTreeSet::new();
    for (_, d, file) in &fns {
        findings.extend(flow::analyze_fn(&prog, &summaries, d, file).findings);
    }
    for f in &findings {
        out.push(diagnostic(f, by_file));
    }
}

/// `/// wj-taint: trusted` with no `(<reason>)` on any function, method or trait method of `items`.
fn malformed_annotations(items: &[Item<'_>], p: &Parsed, out: &mut Vec<Diagnostic>) {
    let report = |name: &str, doc: Option<&str>, out: &mut Vec<Diagnostic>| {
        if flow::pragma_of_doc(doc) == Pragma::TrustedNoReason {
            let ix = SourceIndex::new(p.file.clone(), &p.text);
            let at = Locator::new(&ix).top_level_decl("fn", name, 0).unwrap_or_else(|| ix.span_at(1, 1, 1));
            out.push(
                Diagnostic::error(CODE, format!("`{ANNOTATION} trusted` on `{name}` needs a reason"))
                    .at(at.with_label("annotated without a reason"))
                    .with_help(format!("write `/// {ANNOTATION} trusted(<why this function may use the value>)`: the reason is the review record")),
            );
        }
    };
    for i in items {
        match i {
            Item::Function { decl, .. } => report(&decl.name, decl.doc_comment.as_deref(), out),
            Item::Impl { block, .. } => block.functions.iter().for_each(|f| report(&f.name, f.doc_comment.as_deref(), out)),
            Item::Trait { decl, .. } => decl.methods.iter().for_each(|f| report(&f.name, f.doc_comment.as_deref(), out)),
            Item::Mod { items, .. } => malformed_annotations(items, p, out),
            _ => {}
        }
    }
}

fn span_of(place: &flow::Place, by_file: &HashMap<String, &Parsed>) -> Option<Span> {
    let p = by_file.get(&place.file)?;
    let ix = SourceIndex::new(p.file.clone(), &p.text);
    let loc = Some(crate::source_map::Location { file: place.file.clone().into(), line: place.line, column: place.column });
    if place.line == 0 {
        return None;
    }
    Locator::new(&ix).word(&loc, &place.word).or_else(|| Some(ix.span_at(place.line, place.column.max(1), place.word.len().max(1))))
}

fn diagnostic(f: &Finding, by_file: &HashMap<String, &Parsed>) -> Diagnostic {
    let role = &f.sink.role;
    let mut d = Diagnostic::error(CODE, format!("data from {} reaches {} without being sanitized", f.source_what, role.desc));
    let label = if f.via.is_empty() { role.role.to_string() } else { format!("the value is passed to `{}`, which uses it as {}", f.via.join("` and `"), role.desc) };
    match span_of(&f.report_at, by_file) {
        Some(s) => d = d.at(s.with_label(label)),
        None => d = d.with_note(format!("at {}:{}", f.report_at.file, f.report_at.line)),
    }
    if let Some(s) = span_of(&f.source_at, by_file) {
        d = d.with_secondary(s.with_label(format!("{} is read here", f.source_what)));
    }
    if !f.via.is_empty() {
        d = d.with_note(format!("`{}` is called at {}:{}", f.sink.callee, f.sink.at.file, f.sink.at.line));
    }
    d.with_help(format!("`{}` returns untrusted data; {}", f.source_callee, role.advice))
        .with_note("ADR-0009: untrusted data must not reach a command, a file path or SQL text unsanitized")
}
