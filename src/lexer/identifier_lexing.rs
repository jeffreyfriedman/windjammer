use super::{Lexer, Token};

impl Lexer {
    pub(in crate::lexer) fn read_identifier_string(&mut self) -> String {
        let mut ident = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        ident
    }

    pub(in crate::lexer) fn read_identifier(&mut self) -> Token {
        let ident = self.read_identifier_string();

        // Check for keywords
        match ident.as_str() {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "mut" => Token::Mut,
            "const" => Token::Const,
            "static" => Token::Static,
            "struct" => Token::Struct,
            "enum" => Token::Enum,
            "trait" => Token::Trait,
            "impl" => Token::Impl,
            "match" => Token::Match,
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "while" => Token::While,
            "loop" => Token::Loop,
            "return" => Token::Return,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "use" => Token::Use,
            "mod" => Token::Mod,
            "extern" => Token::Extern,
            "thread" => Token::Thread,
            "async" => Token::Async,
            "await" => Token::Await,
            "defer" => Token::Defer,
            "pub" => Token::Pub,
            "self" => Token::Self_,
            "Self" => Token::Ident("Self".to_string()), // Capital Self is a type, not keyword
            "unsafe" => Token::Unsafe,
            "as" => Token::As,
            "where" => Token::Where,
            "type" => Token::Type,
            "dyn" => Token::Dyn,
            "bound" => Token::Bound,
            "int" => Token::Int,
            "int32" => Token::Int32,
            "uint" => Token::Uint,
            "float" => Token::Float,
            "bool" => Token::Bool,
            "string" => Token::String,
            "true" => Token::BoolLiteral(true),
            "false" => Token::BoolLiteral(false),
            "_" => Token::Underscore,
            _ => Token::Ident(ident),
        }
    }
}
