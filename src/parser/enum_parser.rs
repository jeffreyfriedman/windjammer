// Enum parsing — extracted from item_parser.rs

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_enum(&mut self) -> Result<EnumDecl, String> {
        // Token::Enum already consumed in parse_item

        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected enum name".to_string());
        };

        // Parse type parameters: enum Option<T>, enum Result<T, E>
        let type_params = self.parse_type_params()?;

        self.expect(Token::LBrace)?;

        let mut variants = Vec::new();
        while self.current_token() != &Token::RBrace {
            // Collect doc comment for variant (if any)
            let doc_comment = if let Token::DocComment(comment) = self.current_token() {
                let doc = comment.clone();
                self.advance();
                Some(doc)
            } else {
                None
            };

            let variant_name = match self.current_token() {
                Token::Ident(n) => {
                    let name = n.clone();
                    self.advance();
                    name
                }
                // Allow type-keyword names as variant names (e.g. `string(string)` in EventDataValue)
                Token::String => {
                    self.advance();
                    "string".to_string()
                }
                _ => return Err("Expected variant name".to_string()),
            };

            let data = if self.current_token() == &Token::LParen {
                // Tuple-style variant: Variant(Type1, Type2, Type3)
                self.advance();

                let mut types = Vec::new();

                // Parse types separated by commas
                if self.current_token() != &Token::RParen {
                    loop {
                        types.push(self.parse_type()?);

                        if self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RParen {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }

                self.expect(Token::RParen)?;
                EnumVariantData::Tuple(types)
            } else if self.current_token() == &Token::LBrace {
                // Struct-style variant: Variant { field1: Type1, field2: Type2 }
                self.advance(); // consume {

                let mut fields = Vec::new();

                // Parse field: type pairs
                while self.current_token() != &Token::RBrace && self.current_token() != &Token::Eof
                {
                    // Parse field name
                    let field_name = if let Token::Ident(name) = self.current_token() {
                        let n = name.clone();
                        self.advance();
                        n
                    } else {
                        return Err(format!(
                            "Expected field name in struct variant (at token position {})",
                            self.position
                        ));
                    };

                    self.expect(Token::Colon)?;

                    // Parse field type
                    let field_type = self.parse_type()?;

                    fields.push((field_name, field_type));

                    // Check for comma or end
                    if self.current_token() == &Token::Comma {
                        self.advance();
                        // Allow trailing comma
                        if self.current_token() == &Token::RBrace {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                self.expect(Token::RBrace)?;
                EnumVariantData::Struct(fields)
            } else {
                EnumVariantData::Unit
            };

            variants.push(EnumVariant {
                name: variant_name,
                data,
                doc_comment,
            });

            if self.current_token() == &Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;

        Ok(EnumDecl {
            name,
            is_pub: false, // Will be set by parse_item() if pub keyword present
            type_params,
            variants,
            doc_comment: None, // Set by parse_item if doc comments present
        })
    }
}
