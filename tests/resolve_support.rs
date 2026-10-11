//! Shared helpers for `resolve_*_test.rs` (not itself a test binary).
#![allow(dead_code)]

use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::resolve::{
    resolve_program, CallResolution, DefId, ParsedFile, Reason, Res, ResolvedProgram,
};

pub fn parse(code: &str) -> &'static Program<'static> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    Box::leak(Box::new(parser.parse().expect("parse")))
}

/// Resolve `(module_path, source)` pairs; module path uses `::` and `""` is the root file.
pub fn resolve_files(files: &[(&str, &str)]) -> ResolvedProgram<'static> {
    let parsed: Vec<ParsedFile<'static>> = files
        .iter()
        .map(|(m, src)| {
            let path: Vec<String> = m
                .split("::")
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            ParsedFile::new(path, format!("{m}.wj").into(), parse(src))
        })
        .collect();
    resolve_program(&parsed)
}

pub fn resolve_one(src: &str) -> ResolvedProgram<'static> {
    resolve_files(&[("", src)])
}

pub fn def(rp: &ResolvedProgram<'_>, path: &str) -> DefId {
    rp.defs().find_one(path).unwrap_or_else(|| {
        panic!(
            "no unique def at `{path}`; have {:?}",
            rp.defs().find_path(path)
        )
    })
}

/// The single call to `callee` inside the function at `fn_path`.
pub fn call<'r>(rp: &'r ResolvedProgram<'_>, fn_path: &str, callee: &str) -> &'r CallResolution {
    let hits: Vec<_> = rp
        .calls_in(fn_path)
        .into_iter()
        .filter(|c| c.callee == callee)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "calls to `{callee}` in `{fn_path}`: {hits:?}"
    );
    hits[0]
}

pub fn res<'r>(rp: &'r ResolvedProgram<'_>, fn_path: &str, callee: &str) -> &'r Res {
    &call(rp, fn_path, callee).res
}

#[track_caller]
pub fn assert_def(rp: &ResolvedProgram<'_>, fn_path: &str, callee: &str, target: &str) {
    assert_eq!(
        res(rp, fn_path, callee),
        &Res::Def(def(rp, target)),
        "{callee} in {fn_path}"
    );
}

pub fn is_external(r: &Res) -> bool {
    matches!(r, Res::Unresolved(Reason::External(_)))
}
