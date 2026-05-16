use windjammer::lexer::Lexer;
use std::fs;

fn main() {
    let source = fs::read_to_string("examples/applications/form_validation/main.wj").unwrap();
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    for i in 820..830 {
        if i < tokens.len() {
            println!("{}: {:?}", i, tokens[i]);
        }
    }
}

