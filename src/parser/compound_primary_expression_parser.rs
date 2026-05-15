// Larger primary-expression forms: `match`, closures (`|_|`, `||`), `if` / `if let`, `unsafe` blocks.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn parse_primary_match(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        // Match expression
        self.advance();
        // Parse the value to match on, but don't allow struct literals here
        // (since we need to see the { for the match arms)
        let value = self.parse_match_value()?;

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

            // TDD: Match arms can contain assignments (statements), not just expressions
            // Check if this is a block or a single statement/expression
            let (body, is_block) = if self.current_token() == &Token::LBrace {
                // Block expression: match x { Pattern => { ... } }
                self.advance();
                let statements = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                let block = self.alloc_expr(Expression::Block {
                    statements,
                    is_unsafe: false,
                    location: self.current_location(),
                });
                (block, true)
            } else {
                // Try to parse as statement first (for assignments), then as expression
                let _checkpoint = self.position;

                // Peek ahead to see if this looks like an assignment
                let is_assignment = if let Token::Ident(_) = self.current_token() {
                    // Check for identifier followed by = (or .field = for field assignment)
                    let mut ahead = 1;
                    loop {
                        match self.peek(ahead) {
                            Some(Token::Assign) => break true,
                            Some(Token::Dot) | Some(Token::LBracket) => {
                                ahead += 1;
                                if let Some(Token::Ident(_)) = self.peek(ahead) {
                                    ahead += 1;
                                } else {
                                    break false;
                                }
                            }
                            _ => break false,
                        }
                    }
                } else {
                    false
                };

                // TDD: break/continue/return in match arm (e.g. None => break) must be parsed
                // as statements, not expressions. Expression parser doesn't handle them.
                let is_control_flow = matches!(
                    self.current_token(),
                    Token::Break | Token::Continue | Token::Return
                );

                if is_assignment || is_control_flow {
                    // Parse as statement (assignment or break/continue/return)
                    let stmt = self.parse_statement()?;
                    // Wrap in block expression
                    let block = self.alloc_expr(Expression::Block {
                        statements: vec![stmt],
                        is_unsafe: false,
                        location: self.current_location(),
                    });
                    (block, false)
                } else {
                    // Parse as expression
                    (self.parse_expression()?, false)
                }
            };

            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });

            // TDD: Match arms must be comma-separated
            // Exception: Commas are optional after block expressions (Rust-style)
            if self.current_token() == &Token::Comma {
                self.advance();
                // Allow trailing comma before closing brace
                if self.current_token() == &Token::RBrace {
                    break;
                }
            } else if self.current_token() == &Token::RBrace {
                // End of match arms
                break;
            } else if !is_block {
                // No comma after a non-block expression (and not at end) - this is an error
                return Err(format!(
                    "Expected ',' or '}}' after match arm, got {:?}",
                    self.current_token()
                ));
            }
            // If is_block is true and no comma, continue to next arm (comma is optional)
        }

        self.expect(Token::RBrace)?;

        // Convert match arms into a match expression
        // For now, wrap in a block expression
        let match_stmt = self.alloc_stmt(Statement::Match {
            value,
            arms,
            location: self.current_location(),
        });
        Ok(self.alloc_expr(Expression::Block {
            statements: vec![match_stmt],
            is_unsafe: false,
            location: self.current_location(),
        }))
    }

    pub(in crate::parser) fn parse_primary_closure_pipe(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
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
            self.alloc_expr(Expression::Block {
                statements,
                is_unsafe: false,
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
                self.alloc_expr(Expression::Block {
                    statements: vec![stmt],
                    is_unsafe: false,
                    location: self.current_location(),
                })
            } else {
                // Reset and parse as a normal expression
                self.position = checkpoint;
                self.parse_expression()?
            }
        };

        Ok(self.alloc_expr(Expression::Closure {
            parameters,
            body,
            location: self.current_location(),
        }))
    }

    pub(in crate::parser) fn parse_primary_closure_or(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        // Closure with no parameters: || body
        self.advance(); // consume '||'

        // Parse closure body - can be either an expression or a block
        let body = if self.current_token() == &Token::LBrace {
            // Block closure: || { statements }
            self.advance();
            let statements = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;
            self.alloc_expr(Expression::Block {
                statements,
                is_unsafe: false,
                location: self.current_location(),
            })
        } else {
            // Expression closure: || expr
            self.parse_expression()?
        };

        Ok(self.alloc_expr(Expression::Closure {
            parameters: Vec::new(), // No parameters
            body,
            location: self.current_location(),
        }))
    }

    pub(in crate::parser) fn parse_primary_if(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
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

            // Parse optional guard: `if let Some(x) = opt if x > 0 { ... }`
            let guard = if self.current_token() == &Token::If {
                self.advance();
                Some(self.parse_match_value()?)
            } else {
                None
            };

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

            let then_body = self.alloc_expr(Expression::Block {
                statements: then_block,
                is_unsafe: false,
                location: self.current_location(),
            });

            let mut arms = vec![MatchArm {
                pattern,
                guard,
                body: then_body,
            }];

            if let Some(else_block) = else_block {
                let else_body = self.alloc_expr(Expression::Block {
                    statements: else_block,
                    is_unsafe: false,
                    location: self.current_location(),
                });
                arms.push(MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: else_body,
                });
            }

            let match_stmt = self.alloc_stmt(Statement::Match {
                value: expr,
                arms,
                location: self.current_location(),
            });

            return Ok(self.alloc_expr(Expression::Block {
                statements: vec![match_stmt],
                is_unsafe: false,
                location: self.current_location(),
            }));
        }

        // Regular if expression
        // Use parse_match_value to avoid struct literal ambiguity
        let condition = self.parse_match_value()?;

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
        let if_stmt = self.alloc_stmt(Statement::If {
            condition,
            then_block,
            else_block,
            location: self.current_location(),
        });

        Ok(self.alloc_expr(Expression::Block {
            statements: vec![if_stmt],
            is_unsafe: false,
            location: self.current_location(),
        }))
    }

    pub(in crate::parser) fn parse_primary_unsafe_block(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        // Unsafe block: unsafe { ... }
        self.advance(); // consume 'unsafe'
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        Ok(self.alloc_expr(Expression::Block {
            statements: body,
            is_unsafe: true,
            location: self.current_location(),
        }))
    }
}
