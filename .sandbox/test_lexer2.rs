use windjammer::lexer::Lexer;

fn main() {
    let source = "use std::db\nuse ../models/user::{User, RegisterRequest}";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    
    for (i, token) in tokens.iter().enumerate() {
        println!("{}: {:?}", i, token);
    }
}

