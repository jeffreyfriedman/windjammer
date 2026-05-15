// Expression Parser - Windjammer Expression Parsing Functions
//
// This module contains functions for parsing expressions in Windjammer.
// Expressions include literals, identifiers, binary/unary operations, function calls,
// method calls, field access, indexing, closures, if expressions, match expressions, etc.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<&'static Expression<'static>, String> {
        self.parse_ternary_expression()
    }

    pub(in crate::parser) fn parse_ternary_expression(&mut self) -> Result<&'static Expression<'static>, String> {
        // Ternary operator removed - use if/else expressions instead
        // This simplifies the parser and eliminates ambiguity with TryOp (?)
        self.parse_binary_expression(0)
    }

    pub(crate) fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset).map(|t| &t.token)
    }

    pub(in crate::parser) fn parse_arguments(
        &mut self,
    ) -> Result<Vec<(Option<String>, &'static Expression<'static>)>, String> {
        let mut args = Vec::new();

        while self.current_token() != &Token::RParen {
            // Check for labeled argument: name: expr
            let label = if let Token::Ident(name) = self.current_token() {
                if self.peek(1) == Some(&Token::Colon) {
                    let label = name.clone();
                    self.advance(); // consume identifier
                    self.advance(); // consume colon
                    Some(label)
                } else {
                    None
                }
            } else {
                None
            };

            let expr = self.parse_expression()?;
            args.push((label, expr));

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }
}
