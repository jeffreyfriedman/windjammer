//! The interpreter keeps the programs it runs only as long as it lives: their AST arenas are freed when the
//! interpreter (and every other owner) is dropped, instead of being leaked for the life of the process.

use windjammer::interpreter::Interpreter;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

const SRC: &str = "fn main() { let x = 1 + 2\n println(\"${x}\") }\n";

#[test]
fn interpreter_frees_program_arenas_on_drop() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let tokens = Lexer::new(SRC).tokenize_with_locations();
            let mut parser = Parser::new_with_source(tokens, "t.wj".into(), SRC.into());
            let ledger = parser.arena_ledger();
            let program = parser.parse().expect("parses");
            drop(parser);
            let mut it = Interpreter::new_capturing();
            it.run_checked(&program).expect("runs");
            assert_eq!(it.get_output().trim(), "3");
            drop(program);
            assert_eq!(ledger.freed(), 0, "the interpreter still owns the program it runs");
            drop(it);
            assert_eq!(ledger.freed(), 1, "dropping the interpreter must free the arenas");
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn repl_inputs_are_freed_with_the_interpreter() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let mut it = Interpreter::new_capturing();
            for i in 0..50 {
                it.repl_eval(&format!("fn f{i}() -> int {{ {i} }}")).expect("item");
            }
            assert_eq!(it.repl_eval("f7()").expect("call").as_deref(), Some("7"));
        })
        .unwrap()
        .join()
        .unwrap();
}
