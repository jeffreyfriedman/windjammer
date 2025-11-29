// Expression Parser - Windjammer Expression Parsing Functions
//
// This module contains functions for parsing expressions in Windjammer.
// Expressions include literals, identifiers, binary/unary operations, function calls,
// method calls, field access, indexing, closures, if expressions, match expressions, etc.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_ternary_expression()
    }

    fn parse_ternary_expression(&mut self) -> Result<Expression, String> {
        // Ternary operator removed - use if/else expressions instead
        // This simplifies the parser and eliminates ambiguity with TryOp (?)
        self.parse_binary_expression(0)
    }

    pub(crate) fn parse_match_value(&mut self) -> Result<Expression, String> {
        // Parse a non-struct-literal expression for match values
        // This is basically parse_binary_expression but without struct literal support
        let mut left = match self.current_token() {
            Token::LParen => {
                self.advance();

                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    return Ok(Expression::Tuple {
                        elements: vec![],
                        location: self.current_location(),
                    });
                }

                // Parse the first expression inside parentheses
                // Use parse_match_value recursively to avoid parsing assignment operators
                let first_expr = self.parse_match_value()?;

                // Check if it's a tuple (has comma) or just a parenthesized expression
                if self.current_token() == &Token::Comma {
                    let mut elements = vec![first_expr];

                    while self.current_token() == &Token::Comma {
                        self.advance(); // consume comma

                        // Allow trailing comma
                        if self.current_token() == &Token::RParen {
                            break;
                        }

                        elements.push(self.parse_match_value()?);
                    }

                    self.expect(Token::RParen)?;
                    Expression::Tuple {
                        elements,
                        location: self.current_location(),
                    }
                } else {
                    // Just a parenthesized expression
                    self.expect(Token::RParen)?;
                    first_expr
                }
            }
            Token::LBracket => {
                // Array literal: [a, b, c] or array repeat: [value; count]
                self.advance();

                // Check for empty array []
                if self.current_token() == &Token::RBracket {
                    self.advance();
                    return Ok(Expression::Array {
                        elements: vec![],
                        location: self.current_location(),
                    });
                }

                let first_element = self.parse_expression()?;

                // Check for array repeat syntax: [value; count]
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                    let count = self.parse_expression()?;
                    self.expect(Token::RBracket)?;

                    // Represent as a macro invocation: vec![value; count]
                    return Ok(Expression::MacroInvocation {
                        name: "vec".to_string(),
                        args: vec![first_element, count],
                        delimiter: MacroDelimiter::Brackets,
                        location: self.current_location(),
                    });
                }

                // Regular array literal
                let mut elements = vec![first_element];

                while self.current_token() == &Token::Comma {
                    self.advance(); // consume comma

                    // Allow trailing comma
                    if self.current_token() == &Token::RBracket {
                        break;
                    }

                    elements.push(self.parse_expression()?);
                }

                self.expect(Token::RBracket)?;
                Expression::Array {
                    elements,
                    location: self.current_location(),
                }
            }
            Token::Ampersand => {
                // Handle & and &mut unary operators
                self.advance();
                let is_mut = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: if is_mut {
                        UnaryOp::MutRef
                    } else {
                        UnaryOp::Ref
                    },
                    operand: Box::new(inner),
                    location: self.current_location(),
                }
            }
            Token::Star => {
                // Handle * dereference operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(inner),
                    location: self.current_location(),
                }
            }
            Token::Minus => {
                // Handle - negation operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(inner),
                    location: self.current_location(),
                }
            }
            Token::Bang => {
                // Handle ! not operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(inner),
                    location: self.current_location(),
                }
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();

                // Handle qualified paths with :: (e.g., std::fs::read)
                while self.current_token() == &Token::ColonColon {
                    // Look ahead to see if there's an identifier after ::
                    if self.position + 1 < self.tokens.len() {
                        if let Token::Ident(next_name) = &self.tokens[self.position + 1].token {
                            // This is a qualified path segment
                            qualified_name.push_str("::");
                            qualified_name.push_str(next_name);
                            self.advance(); // consume ::
                            self.advance(); // consume identifier
                        } else if let Token::Lt = &self.tokens[self.position + 1].token {
                            // This is turbofish (e.g., Type::<T>), stop here
                            break;
                        } else {
                            // Unknown token after ::, stop here
                            break;
                        }
                    } else {
                        // No more tokens, stop
                        break;
                    }
                }

                // Don't check for { here - just create the identifier
                // and continue to postfix operators
                Expression::Identifier {
                    name: qualified_name,
                    location: self.current_location(),
                }
            }
            _ => self.parse_primary_expression()?,
        };

        // Handle postfix operators (., [, etc.) before binary operators
        loop {
            match self.current_token() {
                Token::Dot => {
                    // Check for .await
                    if self.peek(1) == Some(&Token::Await) {
                        self.advance(); // consume '.'
                        self.advance(); // consume 'await'
                        left = Expression::Await {
                            expr: Box::new(left),
                            location: self.current_location(),
                        };
                    } else {
                        self.advance();
                        let field = if let Token::Ident(name) = self.current_token() {
                            let name = name.clone();
                            self.advance();
                            name
                        } else {
                            return Err("Expected field name after .".to_string());
                        };
                        left = Expression::FieldAccess {
                            object: Box::new(left),
                            field,
                            location: self.current_location(),
                        };
                    }
                }
                Token::LBracket => {
                    self.advance();

                    // Check for slice syntax: [start..end], [start..], [..end]
                    if self.current_token() == &Token::DotDot {
                        // [..end] - slice from beginning
                        self.advance(); // consume '..'
                        let end = if self.current_token() != &Token::RBracket {
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };
                        self.expect(Token::RBracket)?;

                        // Desugar [..end] to .slice(0, end)
                        let end_expr = end.unwrap_or_else(|| {
                            Box::new(Expression::MethodCall {
                                object: Box::new(left.clone()),
                                method: "len".to_string(),
                                type_args: None,
                                arguments: vec![],
                                location: self.current_location(),
                            })
                        });

                        left = Expression::MethodCall {
                            object: Box::new(left),
                            method: "slice".to_string(),
                            type_args: None,
                            arguments: vec![
                                (
                                    None,
                                    Expression::Literal {
                                        value: Literal::Int(0),
                                        location: self.current_location(),
                                    },
                                ),
                                (None, *end_expr),
                            ],
                            location: self.current_location(),
                        };
                    } else {
                        let start_or_index = Box::new(self.parse_expression()?);

                        // Check if this is a slice or regular index
                        if self.current_token() == &Token::DotDot {
                            // [start..] or [start..end] - slice syntax
                            self.advance(); // consume '..'
                            let end = if self.current_token() != &Token::RBracket {
                                Some(Box::new(self.parse_expression()?))
                            } else {
                                None
                            };
                            self.expect(Token::RBracket)?;

                            // Desugar [start..end] to .slice(start, end)
                            let end_expr = end.unwrap_or_else(|| {
                                Box::new(Expression::MethodCall {
                                    object: Box::new(left.clone()),
                                    method: "len".to_string(),
                                    type_args: None,
                                    arguments: vec![],
                                    location: self.current_location(),
                                })
                            });

                            left = Expression::MethodCall {
                                object: Box::new(left),
                                method: "slice".to_string(),
                                type_args: None,
                                arguments: vec![(None, *start_or_index), (None, *end_expr)],
                                location: self.current_location(),
                            };
                        } else {
                            // Regular index: [i]
                            self.expect(Token::RBracket)?;
                            left = Expression::Index {
                                object: Box::new(left),
                                index: start_or_index,
                                location: self.current_location(),
                            };
                        }
                    }
                }
                Token::ColonColon => {
                    // Handle turbofish and static method calls in match values
                    self.advance(); // consume ::

                    if self.current_token() == &Token::Lt {
                        // Turbofish: expr::<Type>
                        self.advance(); // consume <
                        let mut types = vec![self.parse_type()?];
                        while self.current_token() == &Token::Comma {
                            self.advance();
                            if self.current_token() != &Token::Gt {
                                types.push(self.parse_type()?);
                            }
                        }
                        self.expect(Token::Gt)?;

                        // Expect function call after turbofish
                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            left = Expression::MethodCall {
                                object: Box::new(left),
                                method: String::new(), // Empty method name signals turbofish call
                                type_args: Some(types),
                                arguments,
                                location: self.current_location(),
                            };
                        } else {
                            return Err("Expected '(' after turbofish".to_string());
                        }
                    } else if let Token::Ident(method) = self.current_token() {
                        // Static method or path continuation
                        let method = method.clone();
                        self.advance();

                        // Check for turbofish on this method
                        let type_args = if self.current_token() == &Token::ColonColon {
                            // Peek ahead to see if this is turbofish or path continuation
                            if self.peek(1) == Some(&Token::Lt) {
                                // Turbofish: Type::<T>
                                self.advance(); // consume ::
                                self.advance(); // consume <
                                let mut types = vec![self.parse_type()?];
                                while self.current_token() == &Token::Comma {
                                    self.advance();
                                    if self.current_token() != &Token::Gt {
                                        types.push(self.parse_type()?);
                                    }
                                }
                                self.expect(Token::Gt)?;
                                Some(types)
                            } else {
                                // Not turbofish - don't consume ::, let the loop handle it
                                None
                            }
                        } else {
                            None
                        };

                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            left = Expression::MethodCall {
                                object: Box::new(left),
                                method,
                                type_args,
                                arguments,
                                location: self.current_location(),
                            };
                        } else {
                            // Just a path, treat as field access
                            left = Expression::FieldAccess {
                                object: Box::new(left),
                                field: method,
                                location: self.current_location(),
                            };
                        }
                    } else {
                        return Err("Expected '<' or identifier after '::'".to_string());
                    }
                }
                Token::LParen => {
                    // Function call
                    self.advance();
                    let mut arguments = Vec::new();
                    while self.current_token() != &Token::RParen {
                        let arg = self.parse_expression()?;
                        arguments.push((None, arg));
                        if self.current_token() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    left = Expression::Call {
                        function: Box::new(left),
                        arguments,
                        location: self.current_location(),
                    };
                }
                _ => break,
            }
        }

        // Handle binary operators
        while let Some((op, precedence)) = self.get_binary_op() {
            self.advance();
            let right = self.parse_binary_expression(precedence + 1)?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: self.current_location(),
            };
        }

        Ok(left)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> Result<Expression, String> {
        let mut left = self.parse_primary_expression()?;

        loop {
            // Check for pipe operator: value |> func
            if self.current_token() == &Token::PipeOp {
                self.advance();

                // Parse the right side (function to call)
                let func = self.parse_primary_expression()?;

                // Transform: left |> func becomes func(left)
                left = Expression::Call {
                    function: Box::new(func),
                    arguments: vec![(None, left)], // No label for piped argument
                    location: self.current_location(),
                };
                continue;
            }

            // Check for channel send: ch <- value
            if self.current_token() == &Token::LeftArrow {
                self.advance();
                let value = self.parse_expression()?;
                left = Expression::ChannelSend {
                    channel: Box::new(left),
                    value: Box::new(value),
                    location: self.current_location(),
                };
                continue;
            }

            if let Some((op, precedence)) = self.get_binary_op() {
                if precedence < min_precedence {
                    break;
                }

                self.advance();
                let right = self.parse_binary_expression(precedence + 1)?;

                left = Expression::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                    location: self.current_location(),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn get_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.current_token() {
            Token::Or => Some((BinaryOp::Or, 1)),
            Token::And => Some((BinaryOp::And, 2)),
            Token::Eq => Some((BinaryOp::Eq, 3)),
            Token::Ne => Some((BinaryOp::Ne, 3)),
            Token::Lt => Some((BinaryOp::Lt, 4)),
            Token::Le => Some((BinaryOp::Le, 4)),
            Token::Gt => Some((BinaryOp::Gt, 4)),
            Token::Ge => Some((BinaryOp::Ge, 4)),
            Token::Plus => Some((BinaryOp::Add, 5)),
            Token::Minus => Some((BinaryOp::Sub, 5)),
            Token::Star => Some((BinaryOp::Mul, 6)),
            Token::Slash => Some((BinaryOp::Div, 6)),
            Token::Percent => Some((BinaryOp::Mod, 6)),
            _ => None,
        }
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, String> {
        let mut expr = match self.current_token() {
            Token::Thread => {
                // Check if this is a thread block or a module path
                if self.peek(1) == Some(&Token::LBrace) {
                    // Thread block: thread { ... }
                    self.advance();
                    self.expect(Token::LBrace)?;
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    // Wrap in a statement expression
                    Expression::Block {
                        statements: vec![Statement::Thread {
                            body,
                            location: self.current_location(),
                        }],
                        location: self.current_location(),
                    }
                } else {
                    // Module path like thread::sleep_seconds
                    // Parse as identifier and let postfix operators handle ::
                    let name = "thread".to_string();
                    self.advance();
                    Expression::Identifier {
                        name,
                        location: self.current_location(),
                    }
                }
            }
            Token::Async => {
                // Check if this is an async block or a module path
                if self.peek(1) == Some(&Token::LBrace) {
                    // Async block: async { ... }
                    self.advance();
                    self.expect(Token::LBrace)?;
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    // Wrap in a statement expression
                    Expression::Block {
                        statements: vec![Statement::Async {
                            body,
                            location: self.current_location(),
                        }],
                        location: self.current_location(),
                    }
                } else {
                    // Module path like async::something
                    let name = "async".to_string();
                    self.advance();
                    Expression::Identifier {
                        name,
                        location: self.current_location(),
                    }
                }
            }
            Token::LeftArrow => {
                // Channel receive: <-ch
                self.advance();
                let channel = self.parse_primary_expression()?;
                Expression::ChannelRecv {
                    channel: Box::new(channel),
                    location: self.current_location(),
                }
            }
            Token::Ampersand => {
                // Reference: &expr or &mut expr
                self.advance();
                let is_mut = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: if is_mut {
                        UnaryOp::MutRef
                    } else {
                        UnaryOp::Ref
                    },
                    operand: Box::new(operand),
                    location: self.current_location(),
                }
            }
            Token::Star => {
                // Dereference: *expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(operand),
                    location: self.current_location(),
                }
            }
            Token::Minus => {
                // Negation: -expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    location: self.current_location(),
                }
            }
            Token::Bang => {
                // Logical not: !expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    location: self.current_location(),
                }
            }
            Token::Self_ => {
                // self keyword used in expressions
                self.advance();
                Expression::Identifier {
                    name: "self".to_string(),
                    location: self.current_location(),
                }
            }
            Token::IntLiteral(n) => {
                let n = *n;
                self.advance();
                Expression::Literal {
                    value: Literal::Int(n),
                    location: self.current_location(),
                }
            }
            Token::FloatLiteral(f) => {
                let f = *f;
                self.advance();
                Expression::Literal {
                    value: Literal::Float(f),
                    location: self.current_location(),
                }
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Expression::Literal {
                    value: Literal::String(s),
                    location: self.current_location(),
                }
            }
            Token::CharLiteral(c) => {
                let c = *c;
                self.advance();
                Expression::Literal {
                    value: Literal::Char(c),
                    location: self.current_location(),
                }
            }
            Token::InterpolatedString(parts) => {
                // Convert interpolated string to format! macro call
                let parts = parts.clone();
                self.advance();

                let mut format_string = String::new();
                let mut args = Vec::new();

                for part in parts {
                    match part {
                        crate::lexer::StringPart::Literal(lit) => {
                            format_string.push_str(&lit);
                        }
                        crate::lexer::StringPart::Expression(expr_str) => {
                            format_string.push_str("{}");

                            // Parse the expression string
                            let mut expr_lexer = crate::lexer::Lexer::new(&expr_str);
                            let mut expr_tokens = Vec::new();
                            loop {
                                let tok_with_loc = expr_lexer.next_token_with_location();
                                if tok_with_loc.token == crate::lexer::Token::Eof {
                                    break;
                                }
                                expr_tokens.push(tok_with_loc);
                            }

                            // Parse the tokens into an expression
                            let mut expr_parser = Parser::new(expr_tokens);
                            if let Ok(expr) = expr_parser.parse_expression() {
                                args.push(expr);
                            }
                        }
                    }
                }

                // Create format! macro invocation
                let mut macro_args = vec![Expression::Literal {
                    value: Literal::String(format_string),
                    location: self.current_location(),
                }];
                macro_args.extend(args);

                Expression::MacroInvocation {
                    name: "format".to_string(),
                    args: macro_args,
                    delimiter: MacroDelimiter::Parens,
                    location: self.current_location(),
                }
            }
            Token::BoolLiteral(b) => {
                let b = *b;
                self.advance();
                Expression::Literal {
                    value: Literal::Bool(b),
                    location: self.current_location(),
                }
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();

                // Handle qualified paths with :: (e.g., sqlx::SqlitePool, std::fs::File)
                while self.current_token() == &Token::ColonColon {
                    // Look ahead to see if there's an identifier after ::
                    if self.position + 1 < self.tokens.len() {
                        if let Token::Ident(next_name) = &self.tokens[self.position + 1].token {
                            // This is a qualified path segment
                            qualified_name.push_str("::");
                            qualified_name.push_str(next_name);
                            self.advance(); // consume ::
                            self.advance(); // consume identifier
                        } else if let Token::Lt = &self.tokens[self.position + 1].token {
                            // This is turbofish (e.g., Type::<T>), stop here
                            break;
                        } else {
                            // Unknown token after ::, stop here
                            break;
                        }
                    } else {
                        // No more tokens, stop
                        break;
                    }
                }

                // Check for struct literal
                // Only parse as struct literal if the name looks like a type (starts with uppercase)
                // AND the next tokens look like struct literal syntax (field: value or field,)
                // This avoids ambiguity in contexts like "for item in items { ... }"
                let looks_like_type = qualified_name
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_uppercase());

                let looks_like_struct_literal =
                    if looks_like_type && self.current_token() == &Token::LBrace {
                        // Lookahead: check if the first token after { looks like a field name
                        // followed by : or , or }
                        if self.position + 1 < self.tokens.len() {
                            match &self.tokens[self.position + 1].token {
                                Token::Ident(_) | Token::RBrace => {
                                    // Could be struct literal: { field: ... } or { field, ... } or { }
                                    if self.position + 2 < self.tokens.len() {
                                        matches!(
                                            &self.tokens[self.position + 2].token,
                                            Token::Colon | Token::Comma | Token::RBrace
                                        )
                                    } else {
                                        true
                                    }
                                }
                                _ => false,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                if looks_like_struct_literal {
                    self.advance();
                    let mut fields = Vec::new();

                    while self.current_token() != &Token::RBrace {
                        // Allow identifiers or keywords as field names
                        let field_name = match self.current_token() {
                            Token::Ident(name) => name.clone(),
                            Token::Async => "async".to_string(),
                            Token::Thread => "thread".to_string(),
                            Token::Type => "type".to_string(),
                            Token::Self_ => "self".to_string(),
                            tok => {
                                return Err(format!("Expected field name in struct literal, got {:?} at position {}", tok, self.position));
                            }
                        };
                        self.advance();

                        let field_value = if self.current_token() == &Token::Colon {
                            // Regular syntax: field: value
                            self.advance();
                            self.parse_expression()?
                        } else {
                            // Shorthand syntax: field (implicitly field: field)
                            Expression::Identifier {
                                name: field_name.clone(),
                                location: self.current_location(),
                            }
                        };

                        fields.push((field_name, field_value));

                        if self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RBrace {
                                break;
                            }
                        } else if self.current_token() != &Token::RBrace {
                            return Err(
                                    format!("Expected comma or closing brace in struct literal, got {:?} at position {}", self.current_token(), self.position)
                                );
                        }
                    }

                    self.expect(Token::RBrace)?;
                    Expression::StructLiteral {
                        name: qualified_name,
                        fields,
                        location: self.current_location(),
                    }
                } else {
                    Expression::Identifier {
                        name: qualified_name,
                        location: self.current_location(),
                    }
                }
            }
            Token::LParen => {
                self.advance();

                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    Expression::Tuple {
                        elements: vec![],
                        location: self.current_location(),
                    }
                } else {
                    let first_expr = self.parse_expression()?;

                    // Check if this is a tuple or just a parenthesized expression
                    if self.current_token() == &Token::Comma {
                        // It's a tuple
                        let mut exprs = vec![first_expr];

                        while self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RParen {
                                break;
                            }
                            exprs.push(self.parse_expression()?);
                        }

                        self.expect(Token::RParen)?;
                        Expression::Tuple {
                            elements: exprs,
                            location: self.current_location(),
                        }
                    } else {
                        // Just a parenthesized expression
                        self.expect(Token::RParen)?;
                        first_expr
                    }
                }
            }
            Token::LBracket => {
                // Array literal: [a, b, c] or array repeat: [value; count]
                self.advance();

                // Check for empty array []
                if self.current_token() == &Token::RBracket {
                    self.advance();
                    Expression::Array {
                        elements: vec![],
                        location: self.current_location(),
                    }
                } else {
                    let first_element = self.parse_expression()?;

                    // Check for array repeat syntax: [value; count]
                    if self.current_token() == &Token::Semicolon {
                        self.advance();
                        let count = self.parse_expression()?;
                        self.expect(Token::RBracket)?;

                        // Represent as a macro invocation: vec![value; count]
                        Expression::MacroInvocation {
                            name: "vec".to_string(),
                            args: vec![first_element, count],
                            delimiter: MacroDelimiter::Brackets,
                            location: self.current_location(),
                        }
                    } else {
                        // Regular array literal
                        let mut elements = vec![first_element];

                        while self.current_token() == &Token::Comma {
                            self.advance(); // consume comma

                            // Allow trailing comma
                            if self.current_token() == &Token::RBracket {
                                break;
                            }

                            elements.push(self.parse_expression()?);
                        }

                        self.expect(Token::RBracket)?;
                        Expression::Array {
                            elements,
                            location: self.current_location(),
                        }
                    }
                }
            }
            Token::Match => {
                // Match expression
                self.advance();
                // Parse the value to match on, but don't allow struct literals here
                // (since we need to see the { for the match arms)
                let value = Box::new(self.parse_match_value()?);

                self.expect(Token::LBrace)?;

                let mut arms = Vec::new();
                while self.current_token() != &Token::RBrace {
                    let pattern = self.parse_pattern_with_or()?;

                    // Parse optional guard: if condition
                    let guard = if self.current_token() == &Token::If {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };

                    self.expect(Token::FatArrow)?;
                    let body = self.parse_expression()?;

                    arms.push(MatchArm {
                        pattern,
                        guard,
                        body,
                    });

                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                }

                self.expect(Token::RBrace)?;

                // Convert match arms into a match expression
                // For now, wrap in a block expression
                let match_stmt = Statement::Match {
                    value: *value,
                    arms,
                    location: self.current_location(),
                };
                Expression::Block {
                    statements: vec![match_stmt],
                    location: self.current_location(),
                }
            }
            Token::Pipe => {
                // Closure: |params| body
                self.advance();
                let mut parameters = Vec::new();

                while self.current_token() != &Token::Pipe {
                    // Handle patterns like &x, &mut x, (a, b), or just x
                    let param_name = match self.current_token() {
                        Token::LParen => {
                            // Tuple destructuring: |(a, b)| or |(a, b, c)|
                            self.advance();
                            let mut tuple_parts = Vec::new();

                            while self.current_token() != &Token::RParen {
                                if let Token::Ident(name) = self.current_token() {
                                    tuple_parts.push(name.clone());
                                    self.advance();
                                } else {
                                    return Err(format!(
                                        "Expected identifier in tuple pattern (at token position {})",
                                        self.position
                                    ));
                                }

                                if self.current_token() == &Token::Comma {
                                    self.advance();
                                } else if self.current_token() != &Token::RParen {
                                    return Err(format!(
                                        "Expected ',' or ')' in tuple pattern (at token position {})",
                                        self.position
                                    ));
                                }
                            }

                            self.expect(Token::RParen)?;

                            // Format as tuple pattern: (a, b)
                            format!("({})", tuple_parts.join(", "))
                        }
                        Token::Ampersand => {
                            self.advance();
                            // Skip optional 'mut'
                            if self.current_token() == &Token::Mut {
                                self.advance();
                            }
                            // Get the identifier
                            if let Token::Ident(name) = self.current_token() {
                                let n = name.clone();
                                self.advance();
                                n
                            } else {
                                return Err(
                                    "Expected identifier after & in closure parameter".to_string()
                                );
                            }
                        }
                        Token::Ident(name) => {
                            let n = name.clone();
                            self.advance();
                            n
                        }
                        Token::Underscore => {
                            self.advance();
                            "_".to_string()
                        }
                        _ => {
                            return Err(format!(
                                "Expected parameter name in closure (at token position {})",
                                self.position
                            ));
                        }
                    };

                    parameters.push(param_name);

                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                }

                self.expect(Token::Pipe)?;

                // Parse closure body - can be either an expression or a block
                let body = if self.current_token() == &Token::LBrace {
                    // Block closure: |x| { statements }
                    self.advance();
                    let statements = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    Box::new(Expression::Block {
                        statements,
                        location: self.current_location(),
                    })
                } else {
                    // Expression closure: |x| expr
                    // Check if this looks like a compound assignment (e.g., *c += 1)
                    // by peeking ahead for compound assignment operators
                    let checkpoint = self.position;

                    // Try to parse the left side
                    let _left_expr = self.parse_primary_expression()?;

                    // Check if followed by a compound assignment operator
                    let is_compound_assign = matches!(
                        self.current_token(),
                        Token::PlusAssign
                            | Token::MinusAssign
                            | Token::StarAssign
                            | Token::SlashAssign
                            | Token::PercentAssign
                    );

                    if is_compound_assign {
                        // Reset and parse as a statement
                        self.position = checkpoint;
                        let stmt = self.parse_statement()?;
                        Box::new(Expression::Block {
                            statements: vec![stmt],
                            location: self.current_location(),
                        })
                    } else {
                        // Reset and parse as a normal expression
                        self.position = checkpoint;
                        Box::new(self.parse_expression()?)
                    }
                };

                Expression::Closure {
                    parameters,
                    body,
                    location: self.current_location(),
                }
            }
            Token::Or => {
                // Closure with no parameters: || body
                self.advance(); // consume '||'

                // Parse closure body - can be either an expression or a block
                let body = if self.current_token() == &Token::LBrace {
                    // Block closure: || { statements }
                    self.advance();
                    let statements = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    Box::new(Expression::Block {
                        statements,
                        location: self.current_location(),
                    })
                } else {
                    // Expression closure: || expr
                    Box::new(self.parse_expression()?)
                };

                Expression::Closure {
                    parameters: Vec::new(), // No parameters
                    body,
                    location: self.current_location(),
                }
            }
            Token::If => {
                // If expression: if cond { ... } else { ... }
                // or if let pattern = expr { ... } else { ... }
                self.advance(); // consume 'if'

                // Check for `if let` pattern
                if self.current_token() == &Token::Let {
                    self.advance(); // consume 'let'

                    // Parse pattern
                    let pattern = self.parse_pattern()?;

                    self.expect(Token::Assign)?; // '='

                    // Parse the expression to match against
                    let expr = self.parse_match_value()?;

                    self.expect(Token::LBrace)?;
                    let then_block = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;

                    let else_block = if self.current_token() == &Token::Else {
                        self.advance();
                        self.expect(Token::LBrace)?;
                        let block = self.parse_block_statements()?;
                        self.expect(Token::RBrace)?;
                        Some(block)
                    } else {
                        None
                    };

                    // Desugar `if let` into a match expression
                    // if let pattern = expr { then_block } else { else_block }
                    // becomes:
                    // match expr {
                    //     pattern => { then_block }
                    //     _ => { else_block }
                    // }
                    let mut arms = vec![MatchArm {
                        pattern,
                        guard: None,
                        body: Expression::Block {
                            statements: then_block,
                            location: self.current_location(),
                        },
                    }];

                    if let Some(else_block) = else_block {
                        arms.push(MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Expression::Block {
                                statements: else_block,
                                location: self.current_location(),
                            },
                        });
                    }

                    let match_stmt = Statement::Match {
                        value: expr,
                        arms,
                        location: self.current_location(),
                    };

                    Expression::Block {
                        statements: vec![match_stmt],
                        location: self.current_location(),
                    }
                } else {
                    // Regular if expression
                    // Use parse_match_value to avoid struct literal ambiguity
                    let condition = Box::new(self.parse_match_value()?);

                    self.expect(Token::LBrace)?;
                    let then_block = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;

                    let else_block = if self.current_token() == &Token::Else {
                        self.advance();
                        // Check for else if
                        if self.current_token() == &Token::If {
                            // else if - parse as nested if statement
                            let if_stmt = self.parse_if()?;
                            Some(vec![if_stmt])
                        } else {
                            // else - parse block
                            self.expect(Token::LBrace)?;
                            let block = self.parse_block_statements()?;
                            self.expect(Token::RBrace)?;
                            Some(block)
                        }
                    } else {
                        None
                    };

                    // Convert to expression by wrapping in a block with an if statement
                    // that returns the value
                    let if_stmt = Statement::If {
                        condition: *condition,
                        then_block,
                        else_block,
                        location: self.current_location(),
                    };

                    Expression::Block {
                        statements: vec![if_stmt],
                        location: self.current_location(),
                    }
                }
            }
            Token::Unsafe => {
                // Unsafe block: unsafe { ... }
                self.advance(); // consume 'unsafe'
                self.expect(Token::LBrace)?;
                let body = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                Expression::Block {
                    statements: body,
                    location: self.current_location(),
                }
            }
            Token::LBrace => {
                // Could be block expression or map literal
                // Disambiguate by looking ahead:
                // - { key: value } → map literal
                // - { stmt; stmt } → block
                self.advance(); // consume '{'

                // Check for empty braces
                if self.current_token() == &Token::RBrace {
                    self.advance();
                    // Empty block (not empty map - use HashMap::new() or map{} for that)
                    return Ok(Expression::Block {
                        statements: vec![],
                        location: self.current_location(),
                    });
                }

                // Try to detect map literal by parsing first item
                // Save position in case we need to backtrack
                let checkpoint = self.position;

                // Try parsing as map literal first
                let is_map = if let Ok(_key) = self.parse_ternary_expression() {
                    // If followed by ':', it's a map literal
                    self.current_token() == &Token::Colon
                } else {
                    false
                };

                // Restore position
                self.position = checkpoint;

                if is_map {
                    // Parse as map literal
                    let mut pairs = vec![];

                    loop {
                        if self.current_token() == &Token::RBrace {
                            break;
                        }

                        let key = self.parse_ternary_expression()?;
                        self.expect(Token::Colon)?;
                        let value = self.parse_expression()?;

                        pairs.push((key, value));

                        if self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RBrace {
                                break;
                            }
                        } else if self.current_token() != &Token::RBrace {
                            return Err("Expected ',' or '}' in map literal".to_string());
                        }
                    }

                    self.expect(Token::RBrace)?;
                    Expression::MapLiteral {
                        pairs,
                        location: self.current_location(),
                    }
                } else {
                    // Parse as block expression
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    Expression::Block {
                        statements: body,
                        location: self.current_location(),
                    }
                }
            }
            Token::Return => {
                // Return expression: return expr
                self.advance(); // consume 'return'
                let return_value = if matches!(
                    self.current_token(),
                    Token::Comma | Token::RBrace | Token::Semicolon
                ) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                };
                // Wrap in a block with a return statement
                Expression::Block {
                    statements: vec![Statement::Return {
                        value: return_value.map(|b| *b),
                        location: self.current_location(),
                    }],
                    location: self.current_location(),
                }
            }
            // Allow certain keywords as identifiers in expression context (e.g., HTML attributes)
            Token::For => {
                self.advance();
                Expression::Identifier {
                    name: "for".to_string(),
                    location: self.current_location(),
                }
            }
            Token::Type => {
                self.advance();
                Expression::Identifier {
                    name: "type".to_string(),
                    location: self.current_location(),
                }
            }
            _ => {
                return Err(format!(
                    "Unexpected token in expression: {:?} (at token position {})",
                    self.current_token(),
                    self.position
                ))
            }
        };

        // Handle postfix operators
        loop {
            expr = match self.current_token() {
                Token::Dot => {
                    // Peek ahead to check for .await
                    if self.peek(1) == Some(&Token::Await) {
                        self.advance(); // consume '.'
                        self.advance(); // consume 'await'
                        Expression::Await {
                            expr: Box::new(expr),
                            location: self.current_location(),
                        }
                    } else {
                        self.advance();
                        // Allow keywords as field names (e.g., std.thread, std.async)
                        let field_opt = match self.current_token() {
                            Token::Ident(f) => Some(f.clone()),
                            Token::Thread => Some("thread".to_string()),
                            Token::Async => Some("async".to_string()),
                            _ => None,
                        };
                        if let Some(field) = field_opt {
                            self.advance();

                            // Check for turbofish ::<Type>
                            let type_args = if self.current_token() == &Token::ColonColon {
                                // Peek ahead to see if this is turbofish
                                if self.peek(1) == Some(&Token::Lt) {
                                    self.advance(); // consume ::
                                    self.advance(); // consume <
                                    let mut types = vec![self.parse_type()?];
                                    while self.current_token() == &Token::Comma {
                                        self.advance();
                                        if self.current_token() != &Token::Gt {
                                            types.push(self.parse_type()?);
                                        }
                                    }
                                    self.expect(Token::Gt)?;
                                    Some(types)
                                } else {
                                    // Not turbofish - don't consume ::
                                    None
                                }
                            } else {
                                None
                            };

                            if self.current_token() == &Token::LParen {
                                // Check for newline before LParen (ASI)
                                if self.had_newline_before_current() {
                                    // ASI: This LParen starts a new statement, not a method call
                                    // Create a field access and break
                                    Expression::FieldAccess {
                                        object: Box::new(expr),
                                        field,
                                        location: self.current_location(),
                                    }
                                } else {
                                    // Method call (possibly with turbofish)
                                    self.advance();
                                    let arguments = self.parse_arguments()?;
                                    self.expect(Token::RParen)?;
                                    Expression::MethodCall {
                                        object: Box::new(expr),
                                        method: field,
                                        type_args,
                                        arguments,
                                        location: self.current_location(),
                                    }
                                }
                            } else if type_args.is_some() {
                                return Err(
                                    "Turbofish syntax only allowed on method calls".to_string()
                                );
                            } else {
                                // Field access
                                Expression::FieldAccess {
                                    object: Box::new(expr),
                                    field,
                                    location: self.current_location(),
                                }
                            }
                        } else {
                            return Err(format!(
                                "Expected field or method name (at token position {})",
                                self.position
                            ));
                        }
                    }
                }
                Token::ColonColon => {
                    // Turbofish on function/static method: func::<Type>() or Type::method::<T>()
                    self.advance();
                    if self.current_token() == &Token::Lt {
                        self.advance();
                        let mut types = vec![self.parse_type()?];
                        while self.current_token() == &Token::Comma {
                            self.advance();
                            if self.current_token() != &Token::Gt {
                                types.push(self.parse_type()?);
                            }
                        }
                        self.expect(Token::Gt)?;

                        // Now expect either () for call, :: for path continuation, or identifier
                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            // Convert to method call with turbofish
                            // For func::<T>(), treat as a special method call on the function
                            Expression::MethodCall {
                                object: Box::new(expr),
                                method: String::new(), // Empty method name signals turbofish call
                                type_args: Some(types),
                                arguments,
                                location: self.current_location(),
                            }
                        } else if self.current_token() == &Token::ColonColon {
                            // Vec::<int>::new() - another :: after turbofish
                            // Continue parsing in the loop, the :: will be handled on next iteration
                            // We need to represent this as a turbofish-qualified path
                            // For now, convert the types to a string and append to the identifier
                            let mut type_str = String::new();
                            type_str.push_str("::<");
                            for (i, ty) in types.iter().enumerate() {
                                if i > 0 {
                                    type_str.push_str(", ");
                                }
                                type_str.push_str(&self.type_to_string(ty));
                            }
                            type_str.push('>');

                            // Update the expression to include the turbofish
                            if let Expression::Identifier { name, .. } = expr {
                                expr = Expression::Identifier {
                                    name: format!("{}{}", name, type_str),
                                    location: self.current_location(),
                                };
                            } else {
                                return Err(
                                    "Turbofish can only be applied to identifiers".to_string()
                                );
                            }
                            // Continue the loop to handle the next ::
                            continue;
                        } else if let Token::Ident(method) = self.current_token() {
                            // Type::method or module::function continuation
                            let method = method.clone();
                            self.advance();
                            Expression::MethodCall {
                                object: Box::new(expr),
                                method,
                                type_args: None,
                                arguments: vec![],
                                location: self.current_location(),
                            }
                        } else {
                            return Err(format!(
                                "Expected '(', '::', or identifier after '::<Type>', got {:?}",
                                self.current_token()
                            ));
                        }
                    } else {
                        // Allow keywords as identifiers after :: (e.g., std::thread, std::async)
                        let method = match self.current_token() {
                            Token::Ident(n) => n.clone(),
                            Token::Thread => "thread".to_string(),
                            Token::Async => "async".to_string(),
                            Token::Await => "await".to_string(),
                            _ => {
                                return Err(format!(
                                    "Expected '<' or identifier after '::', got {:?}",
                                    self.current_token()
                                ));
                            }
                        };
                        self.advance();

                        // Check for turbofish on this method
                        let type_args = if self.current_token() == &Token::ColonColon {
                            // Peek ahead to see if this is turbofish
                            if self.peek(1) == Some(&Token::Lt) {
                                self.advance(); // consume ::
                                self.advance(); // consume <
                                let mut types = vec![self.parse_type()?];
                                while self.current_token() == &Token::Comma {
                                    self.advance();
                                    if self.current_token() != &Token::Gt {
                                        types.push(self.parse_type()?);
                                    }
                                }
                                self.expect(Token::Gt)?;
                                Some(types)
                            } else {
                                // Not turbofish - don't consume ::
                                None
                            }
                        } else {
                            None
                        };

                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            Expression::MethodCall {
                                object: Box::new(expr),
                                method,
                                type_args,
                                arguments,
                                location: self.current_location(),
                            }
                        } else {
                            // Just a path, treat as field access
                            Expression::FieldAccess {
                                object: Box::new(expr),
                                field: method,
                                location: self.current_location(),
                            }
                        }
                    }
                }
                Token::LParen => {
                    // Check for newline before LParen (automatic semicolon insertion)
                    // If there was a newline, this might be a new statement, not a function call
                    if self.had_newline_before_current() {
                        // ASI: Treat newline as statement terminator
                        // Don't consume the LParen - it belongs to the next statement
                        break;
                    }
                    
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(Token::RParen)?;
                    Expression::Call {
                        function: Box::new(expr),
                        arguments,
                        location: self.current_location(),
                    }
                }
                Token::Question => {
                    // TryOp: expr?
                    // No ambiguity since we removed ternary operator
                    self.advance();
                    Expression::TryOp {
                        expr: Box::new(expr),
                        location: self.current_location(),
                    }
                }
                Token::LBracket => {
                    self.advance();

                    // Check for slice syntax: [start..end], [start..], [..end]
                    if self.current_token() == &Token::DotDot {
                        // [..end] - slice from beginning
                        self.advance(); // consume '..'
                        let end = if self.current_token() != &Token::RBracket {
                            Some(Box::new(self.parse_expression()?))
                        } else {
                            None
                        };
                        self.expect(Token::RBracket)?;

                        // Desugar [..end] to .slice(0, end)
                        let end_expr = end.unwrap_or_else(|| {
                            Box::new(Expression::MethodCall {
                                object: Box::new(expr.clone()),
                                method: "len".to_string(),
                                type_args: None,
                                arguments: vec![],
                                location: self.current_location(),
                            })
                        });

                        Expression::MethodCall {
                            object: Box::new(expr),
                            method: "slice".to_string(),
                            type_args: None,
                            arguments: vec![
                                (
                                    None,
                                    Expression::Literal {
                                        value: Literal::Int(0),
                                        location: self.current_location(),
                                    },
                                ),
                                (None, *end_expr),
                            ],
                            location: self.current_location(),
                        }
                    } else {
                        // Parse the first expression
                        // We need to parse without consuming .. as a range operator
                        // So we manually parse a binary expression that stops at ..
                        let start_or_index = self.parse_binary_expression(0)?;

                        // Check if this is a slice or regular index
                        if self.current_token() == &Token::DotDot {
                            // [start..] or [start..end] - slice syntax
                            self.advance(); // consume '..'
                            let end = if self.current_token() != &Token::RBracket {
                                Some(Box::new(self.parse_expression()?))
                            } else {
                                None
                            };
                            self.expect(Token::RBracket)?;

                            // Desugar [start..end] to .slice(start, end)
                            let end_expr = end.unwrap_or_else(|| {
                                Box::new(Expression::MethodCall {
                                    object: Box::new(expr.clone()),
                                    method: "len".to_string(),
                                    type_args: None,
                                    arguments: vec![],
                                    location: self.current_location(),
                                })
                            });

                            Expression::MethodCall {
                                object: Box::new(expr),
                                method: "slice".to_string(),
                                type_args: None,
                                arguments: vec![(None, start_or_index), (None, *end_expr)],
                                location: self.current_location(),
                            }
                        } else {
                            // Regular index: [i]
                            self.expect(Token::RBracket)?;
                            Expression::Index {
                                object: Box::new(expr),
                                index: Box::new(start_or_index),
                                location: self.current_location(),
                            }
                        }
                    }
                }
                Token::DotDot | Token::DotDotEq => {
                    // Don't parse as range if followed by ] (that's slice syntax)
                    if let Some(next_tok) = self.peek(1) {
                        if next_tok == &Token::RBracket {
                            // This is slice syntax like [1..], not a range
                            // Let the LBracket handler deal with it
                            break;
                        }
                    }

                    let inclusive = self.current_token() == &Token::DotDotEq;
                    self.advance();
                    let end = self.parse_primary_expression()?;
                    Expression::Range {
                        start: Box::new(expr),
                        end: Box::new(end),
                        inclusive,
                        location: self.current_location(),
                    }
                }
                Token::As => {
                    self.advance();
                    let type_ = self.parse_type()?;
                    Expression::Cast {
                        expr: Box::new(expr),
                        type_,
                        location: self.current_location(),
                    }
                }
                Token::Bang => {
                    // Macro invocation: name!(...) or name![...] or name!{...}
                    if let Expression::Identifier { name, .. } = expr {
                        self.advance(); // consume '!'

                        let (delimiter, end_token) = match self.current_token() {
                            Token::LParen => (MacroDelimiter::Parens, Token::RParen),
                            Token::LBracket => (MacroDelimiter::Brackets, Token::RBracket),
                            Token::LBrace => (MacroDelimiter::Braces, Token::RBrace),
                            _ => return Err("Expected (, [, or { after macro name!".to_string()),
                        };

                        self.advance(); // consume opening delimiter

                        let mut args = Vec::new();
                        while self.current_token() != &end_token {
                            args.push(self.parse_expression()?);

                            if self.current_token() == &Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }

                        self.expect(end_token)?;

                        Expression::MacroInvocation {
                            name,
                            args,
                            delimiter,
                            location: self.current_location(),
                        }
                    } else {
                        // Not a macro invocation, break out of postfix loop
                        break;
                    }
                }
                _ => break,
            };
        }

        Ok(expr)
    }

    pub(crate) fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset).map(|t| &t.token)
    }

    fn parse_arguments(&mut self) -> Result<Vec<(Option<String>, Expression)>, String> {
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
