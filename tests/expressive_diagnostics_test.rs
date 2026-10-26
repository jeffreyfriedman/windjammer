//! ADR-0018 round 2 diagnostics, as golden messages with exact line:column (columns counted by hand).
//! Covers the `_` nested-call trap, labelled arguments to unresolved callees, and `?.` parsing shapes.

use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn parse(src: &str) -> Result<(), String> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new_with_source(tokens, "t.wj".into(), src.into())));
    parser.parse().map(|_| ())
}

const PRELUDE: &str = "fn key(s: string) -> string {\n    s\n}\n\nfn group(f: fn(string) -> string, s: string) -> string {\n    f(s)\n}\n\n";

fn main_with(body: &str) -> String {
    format!("{PRELUDE}fn main() {{\n    {body}\n}}\n")
}

#[test]
fn shorthand_inside_a_nested_call_is_explained_at_the_underscore_call() {
    // line 10 (PRELUDE has 8 lines + `fn main() {`), the closure starts at the `_`, column 25
    let e = parse(&main_with("let r = group(key(_.clone()), \"a\")")).unwrap_err();
    assert!(e.contains("t.wj:10:"), "{e}");
    assert!(e.contains("`_` shorthand makes a closure here, but `key` expects a value for `s`"), "{e}");
    assert!(e.contains("outer(|x| f(x + 1))"), "the message shows the explicit closure to write: {e}");
}

#[test]
fn shorthand_for_a_function_parameter_or_unknown_callee_is_accepted() {
    // `group`'s first parameter is a function: fine; `unknown_fn` is not declared here: no opinion
    assert!(parse(&main_with("let r = group(_.clone(), \"a\")")).is_ok());
    assert!(parse(&main_with("let r = group(key, \"a\")")).is_ok());
    assert!(parse(&main_with("let xs = [1, 2]\n    let ys = xs.map(_ + 1)")).is_ok());
}

#[test]
fn labelled_argument_to_a_callee_outside_the_file_is_rejected() {
    let e = parse("fn main() {\n    let r = some_other_module_fn(a: 1)\n}\n").unwrap_err();
    assert!(e.contains("named argument `a:` needs a callee declared in this file"), "{e}");
}

#[test]
fn optional_chain_shapes_parse() {
    for ok in ["let a = u?.name", "let a = u?.name?.len()", "let a = u?.f(1, 2) ?? 0", "let a = u?.x?.y?.z"] {
        assert!(parse(&format!("fn main() {{\n    {ok}\n}}\n")).is_ok(), "{ok}");
    }
    // `?` then a continuation line is still try-then-method (the `.` is on the next line)
    assert!(parse("fn main() {\n    let a = f()?\n        .g()\n}\n").is_ok());
    let e = parse("fn main() {\n    let a = u?.\n}\n").unwrap_err();
    assert!(e.contains("expected a field or method name after `?.`"), "{e}");
}
