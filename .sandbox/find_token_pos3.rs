use windjammer::lexer::Lexer;
use std::fs;

fn main() {
    let source = fs::read_to_string("examples/wjfind/benches/comparison_benchmark.wj").unwrap();
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    for i in 342..352 {
        if i < tokens.len() {
            println!("{}: {:?}", i, tokens[i]);
        }
    }
}

