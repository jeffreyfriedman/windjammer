// Struct parsing — extracted from item_parser.rs

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_struct(&mut self, is_extern: bool) -> Result<StructDecl<'static>, String> {
        // Token::Struct already consumed in parse_item

        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected struct name".to_string());
        };

        // Parse type parameters: struct Box<T> { ... }
        let type_params = self.parse_type_params()?;

        // Parse where clause (optional): where T: Clone, U: Debug
        let where_clause = self.parse_where_clause()?;

        // Check for tuple struct: struct Name(T1, T2)
        if self.current_token() == &Token::LParen {
            self.advance(); // consume '('
            let mut tuple_types = Vec::new();
            while self.current_token() != &Token::RParen {
                let is_pub = if self.current_token() == &Token::Pub {
                    self.advance();
                    true
                } else {
                    false
                };
                let _ = is_pub; // visibility tracked but not used in tuple field types
                let field_type = self.parse_type()?;
                tuple_types.push(field_type);
                if self.current_token() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
            return Ok(StructDecl {
                name,
                is_pub: false,
                is_extern,
                type_params,
                where_clause,
                fields: Vec::new(),
                tuple_fields: Some(tuple_types),
                decorators: Vec::new(),
                doc_comment: None,
            });
        }

        // Check for unit struct: struct Name;
        let fields = if self.current_token() == &Token::Semicolon {
            // Unit struct with no fields
            self.advance(); // consume semicolon
            Vec::new()
        } else {
            // Regular struct with fields
            self.expect(Token::LBrace)?;

            let mut fields = Vec::new();
            while self.current_token() != &Token::RBrace {
                // Collect doc comment for field (if any)
                let field_doc_comment = if let Token::DocComment(comment) = self.current_token() {
                    let doc = comment.clone();
                    self.advance();
                    Some(doc)
                } else {
                    None
                };

                // Parse decorators on fields
                let mut field_decorators = Vec::new();
                while let Token::Decorator(_dec_name) = self.current_token() {
                    let decorator = self.parse_decorator()?;
                    field_decorators.push(decorator);
                }

                // Parse pub keyword for fields
                let is_pub = if self.current_token() == &Token::Pub {
                    self.advance();
                    true
                } else {
                    false
                };

                let field_name = if let Token::Ident(n) = self.current_token() {
                    let name = n.clone();
                    self.advance();
                    name
                } else {
                    return Err("Expected field name".to_string());
                };

                self.expect(Token::Colon)?;
                let field_type = self.parse_type()?;

                fields.push(StructField {
                    name: field_name,
                    field_type,
                    decorators: field_decorators,
                    is_pub,
                    doc_comment: field_doc_comment,
                });

                if self.current_token() == &Token::Comma {
                    self.advance();
                }
            }

            self.expect(Token::RBrace)?;
            fields
        };

        Ok(StructDecl {
            name,
            is_pub: false, // Will be set by parse_item() if pub keyword present
            is_extern,
            type_params,
            where_clause,
            fields,
            tuple_fields: None,
            decorators: Vec::new(),
            doc_comment: None, // Set by parse_item if doc comments present
        })
    }
}
