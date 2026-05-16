use windjammer::lexer::Lexer;
use std::fs;

fn main() {
    let source = fs::read_to_string("examples/taskflow/windjammer/src/db/tasks.wj").unwrap();
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    for i in 612..622 {
        if i < tokens.len() {
            println!("{}: {:?}", i, tokens[i]);
        }
    }
}

