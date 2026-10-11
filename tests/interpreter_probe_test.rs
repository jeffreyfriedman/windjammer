#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! In-process driver used by `scripts/interpreter_coverage.py --driver` to measure the
//! interpreter over many programs without paying a process start per file in debug builds.
//! Ignored by default: it needs `WJ_PROBE_FILE`.

use windjammer::interpreter::runner::{exit_code_for, with_big_stack};
use windjammer::interpreter::Interpreter;

#[test]
#[ignore]
fn probe_one() {
    let path = std::env::var("WJ_PROBE_FILE").expect("WJ_PROBE_FILE");
    let (out, code, err) = with_big_stack(move || {
        let src = std::fs::read_to_string(&path).expect("read");
        let mut lexer = windjammer::lexer::Lexer::new(&src);
        let tokens = lexer.tokenize_with_locations();
        // the parser owns the AST arenas: keep it alive for the whole run
        let parser: &'static mut windjammer::parser::Parser =
            Box::leak(Box::new(windjammer::parser::Parser::new_with_source(tokens, path.clone(), src.clone())));
        let program = match parser.parse() {
            Ok(p) => p,
            Err(e) => return (String::new(), 1, format!("parse error: {}", e)),
        };
        let mut it = Interpreter::new_capturing();
        let r = it.run_checked(&program);
        match r {
            Ok(_) => (it.get_output(), 0, String::new()),
            Err(e) => (it.get_output(), exit_code_for(&e), e.render()),
        }
    });
    println!("@@STDOUT-BEGIN");
    print!("{}", out);
    println!("@@STDOUT-END");
    println!("@@EXIT {}", code);
    println!("@@ERR {}", err.replace('\n', "\\n"));
}
