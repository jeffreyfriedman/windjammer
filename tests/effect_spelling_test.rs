//! ADR-0021: a module-dot call (`fs.write(..)` after `use std::fs`) performs exactly the effects of the path call
//! (`fs::write(..)`), for every row of the sink table. Both spellings go through one resolution
//! (`resolve::walk::module_dot_call`), so the effect pass and the taint pass classify them alike.

use std::collections::BTreeSet;
use windjammer::ir::effect_facts::{analyze, sinks, EffectKind};

fn main_kinds(src: &str) -> Option<BTreeSet<EffectKind>> {
    let mut lexer = windjammer::lexer::Lexer::new(src);
    let tokens = lexer.tokenize_with_locations();
    let parser: &'static mut windjammer::parser::Parser = Box::leak(Box::new(windjammer::parser::Parser::new_with_source(tokens, "t.wj".into(), src.to_string())));
    let program: &'static _ = Box::leak(Box::new(parser.parse().ok()?));
    let files = [windjammer::resolve::ParsedFile::new(vec![], "t.wj".into(), program)];
    let r = windjammer::resolve::resolve_program(&files);
    let facts = analyze(&r);
    facts.fns.values().find(|f| f.path == "main").map(|f| f.kinds())
}

fn program(module: &str, f: &str, dot: bool) -> String {
    let call = if dot { format!("{module}.{f}()") } else { format!("{module}::{f}()") };
    format!("use std::{module}\n\nfn main() {{\n    let _ = {call}\n}}\n")
}

#[test]
fn every_sink_row_has_the_same_effects_in_both_spellings() {
    let mut checked = 0;
    let mut problems = Vec::new();
    for path in sinks::all_paths() {
        let Some((module, f)) = path.split_once("::") else { continue };
        if !module.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
            continue; // `TcpStream::connect`: a type's function, not a module's
        }
        let by_path = main_kinds(&program(module, f, false));
        let by_dot = main_kinds(&program(module, f, true));
        let (Some(by_path), Some(by_dot)) = (by_path.clone(), by_dot.clone()) else {
            // `random::bool`, `random::float`: the function's name is a type keyword, neither spelling parses
            if by_path.is_some() != by_dot.is_some() {
                problems.push(format!("{path}: only one spelling parses"));
            }
            continue;
        };
        checked += 1;
        if by_path.is_empty() {
            problems.push(format!("{path}: the path spelling performs nothing: the test program is wrong"));
        }
        if by_path != by_dot {
            problems.push(format!("{module}.{f}: dot spelling performs {by_dot:?}, path spelling {by_path:?}"));
        }
    }
    assert!(checked >= 60, "only {checked} sink rows were exercised");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_local_named_like_a_module_is_not_a_module_call() {
    let src = "use std::fs\n\nstruct W {}\n\nimpl W {\n    fn write(self) {}\n}\n\nfn main() {\n    let fs = W {}\n    fs.write()\n}\n";
    assert_eq!(main_kinds(src), Some(BTreeSet::new()), "`fs` is a local here, so `fs.write()` is a method call");
}
