// Statement Parser - Windjammer Statement Parsing Functions
//
// This module contains functions for parsing statements in Windjammer.
// Statements include let bindings, if/else, match, loops, for loops, while loops,
// return statements, thread blocks, async blocks, defer statements, etc.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_block_statements(
        &mut self,
    ) -> Result<Vec<&'static Statement<'static>>, String> {
        let mut statements = Vec::new();

        while self.current_token() != &Token::RBrace && self.current_token() != &Token::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    pub(crate) fn parse_statement(&mut self) -> Result<&'static Statement<'static>, String> {
        match self.current_token() {
            Token::Let => self.parse_let(),
            Token::Const => self.parse_const_statement(),
            Token::Static => self.parse_static_statement(),
            Token::Return => self.parse_return(),
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::For => self.parse_for(),
            Token::Loop => self.parse_loop(),
            Token::While => self.parse_while(),
            Token::Thread => {
                // Check if this is a thread block or a module path (thread::...)
                if self.peek(1) == Some(&Token::LBrace) {
                    self.parse_thread()
                } else {
                    // It's an expression like thread::spawn() or thread::sleep()
                    let expr = self.parse_expression()?;
                    if self.current_token() == &Token::Semicolon {
                        self.advance();
                    }
                    Ok(self.alloc_stmt(Statement::Expression {
                        expr,
                        location: self.current_location(),
                    }))
                }
            }
            Token::Async => {
                // Check if this is an async block or a module path (async::...)
                if self.peek(1) == Some(&Token::LBrace) {
                    self.parse_async()
                } else {
                    // It's an expression like async::something()
                    let expr = self.parse_expression()?;
                    if self.current_token() == &Token::Semicolon {
                        self.advance();
                    }
                    Ok(self.alloc_stmt(Statement::Expression {
                        expr,
                        location: self.current_location(),
                    }))
                }
            }
            Token::Defer => self.parse_defer(),
            Token::Break => {
                self.advance();
                let stmt = self.alloc_stmt(Statement::Break {
                    location: self.current_location(),
                });
                // Consume trailing semicolon if present
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                }
                Ok(stmt)
            }
            Token::Continue => {
                self.advance();
                let stmt = self.alloc_stmt(Statement::Continue {
                    location: self.current_location(),
                });
                // Consume trailing semicolon if present
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                }
                Ok(stmt)
            }
            Token::Use => {
                self.advance(); // consume 'use'
                let (path, alias) = self.parse_use()?;
                Ok(self.alloc_stmt(Statement::Use {
                    path,
                    alias,
                    is_pub: false, // Statements are never pub
                    location: self.current_location(),
                }))
            }
            _ => {
                // Try to parse as expression first
                let expr = self.parse_expression()?;

                // Check if this is an assignment (expr = value) or compound assignment (expr += value)
                match self.current_token() {
                    Token::Assign => {
                        self.advance(); // consume '='
                        let value = self.parse_expression()?;

                        // Optionally consume semicolon
                        if self.current_token() == &Token::Semicolon {
                            self.advance();
                        }

                        Ok(self.alloc_stmt(Statement::Assignment {
                            target: expr,
                            value,
                            compound_op: None,
                            location: self.current_location(),
                        }))
                    }
                    Token::PlusAssign
                    | Token::MinusAssign
                    | Token::StarAssign
                    | Token::SlashAssign
                    | Token::PercentAssign => {
                        let op_token = self.current_token().clone();
                        self.advance(); // consume compound operator

                        let rhs = self.parse_expression()?;

                        // PRESERVE compound operator for idiomatic Rust output
                        // Map token to CompoundOp
                        let compound_op = match op_token {
                            Token::PlusAssign => CompoundOp::Add,
                            Token::MinusAssign => CompoundOp::Sub,
                            Token::StarAssign => CompoundOp::Mul,
                            Token::SlashAssign => CompoundOp::Div,
                            Token::PercentAssign => CompoundOp::Mod,
                            _ => unreachable!(),
                        };

                        // Optionally consume semicolon
                        if self.current_token() == &Token::Semicolon {
                            self.advance();
                        }

                        Ok(self.alloc_stmt(Statement::Assignment {
                            target: expr,
                            value: rhs, // Just the RHS, not expanded binary expression
                            compound_op: Some(compound_op),
                            location: self.current_location(),
                        }))
                    }
                    _ => {
                        // Optionally consume semicolon after expression statement
                        if self.current_token() == &Token::Semicolon {
                            self.advance();
                        }
                        Ok(self.alloc_stmt(Statement::Expression {
                            expr,
                            location: self.current_location(),
                        }))
                    }
                }
            }
        }
    }

    fn parse_const_statement(&mut self) -> Result<&'static Statement<'static>, String> {
        self.advance(); // consume 'const'
        let (name, type_, value) = self.parse_const_or_static()?;
        Ok(self.alloc_stmt(Statement::Const {
            name,
            type_,
            value,
            location: self.current_location(),
        }))
    }

    fn parse_static_statement(&mut self) -> Result<&'static Statement<'static>, String> {
        self.advance(); // consume 'static'
        let mutable = if self.current_token() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        let (name, type_, value) = self.parse_const_or_static()?;
        Ok(self.alloc_stmt(Statement::Static {
            name,
            mutable,
            type_,
            value,
            location: self.current_location(),
        }))
    }

    fn parse_for(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::For)?;

        // Parse for-loop pattern: use the general pattern parser which handles all cases:
        // - Identifier: for x in ...
        // - Wildcard: for _ in ...
        // - Tuple: for (i, item) in ...
        // - Reference: for &x in ...
        // TDD FIX: Previously only handled Ident, LParen, and Ampersand explicitly,
        // missing Token::Underscore (wildcard patterns like `for _ in 0..3`).
        let pattern = if self.current_token() == &Token::Ampersand {
            // Reference pattern: &x — handle separately because parse_pattern
            // doesn't know about reference patterns in for-loops
            self.advance(); // consume &
            if let Token::Ident(name) = self.current_token() {
                let name = name.clone();
                self.advance();
                Pattern::Reference(self.alloc_pattern(Pattern::Identifier(name)))
            } else {
                return Err("Expected identifier after & in for loop pattern".to_string());
            }
        } else {
            // Use general pattern parser for all other cases (identifier, wildcard, tuple, etc.)
            self.parse_pattern()?
        };

        self.expect(Token::In)?;
        let iterable = self.parse_expression()?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(self.alloc_stmt(Statement::For {
            pattern,
            iterable,
            body,
            location: self.current_location(),
        }))
    }

    fn parse_thread(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::Thread)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(self.alloc_stmt(Statement::Thread {
            body,
            location: self.current_location(),
        }))
    }

    fn parse_async(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::Async)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(self.alloc_stmt(Statement::Async {
            body,
            location: self.current_location(),
        }))
    }

    fn parse_defer(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::Defer)?;
        let stmt = self.parse_statement()?;

        Ok(self.alloc_stmt(Statement::Defer {
            statement: stmt,
            location: self.current_location(),
        }))
    }

    fn parse_let(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::Let)?;

        let mutable = if self.current_token() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };

        // Parse pattern - always use parse_pattern() to handle all cases
        let pattern = self.parse_pattern()?;

        // Check if the pattern is refutable (can fail to match)
        let is_refutable = Self::is_pattern_refutable(&pattern);

        let type_ = if self.current_token() == &Token::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;

        // Check for `else` block (required for refutable patterns)
        let else_block = if self.current_token() == &Token::Else {
            self.advance();
            // Parse the else block (must be a block, not an expression)
            self.expect(Token::LBrace)?;
            let block = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;
            Some(block)
        } else {
            None
        };

        // Refutable patterns require an else block (let-else syntax)
        if is_refutable && else_block.is_none() {
            return Err(format!(
                "Refutable pattern in `let` binding requires an `else` block. Use `let {} = value else {{ ... }}`",
                Self::pattern_to_string(&pattern)
            ));
        }

        // Optionally consume semicolon (semicolons are optional in Windjammer)
        if self.current_token() == &Token::Semicolon {
            self.advance();
        }

        Ok(self.alloc_stmt(Statement::Let {
            pattern,
            mutable,
            type_,
            value,
            else_block,
            location: self.current_location(),
        }))
    }

    fn parse_return(&mut self) -> Result<&'static Statement<'static>, String> {
        self.advance();

        let stmt = if matches!(self.current_token(), Token::RBrace | Token::Semicolon) {
            self.alloc_stmt(Statement::Return {
                value: None,
                location: self.current_location(),
            })
        } else {
            let value = self.parse_expression()?;
            self.alloc_stmt(Statement::Return {
                value: Some(value),
                location: self.current_location(),
            })
        };

        // Consume trailing semicolon if present
        if self.current_token() == &Token::Semicolon {
            self.advance();
        }

        Ok(stmt)
    }

    pub(crate) fn parse_if(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::If)?;

        // Check for `if let` pattern matching
        if self.current_token() == &Token::Let {
            self.advance(); // consume 'let'

            // Parse pattern
            let pattern = self.parse_pattern()?;

            self.expect(Token::Assign)?;

            // Parse value to match against
            let value = self.parse_expression()?;

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

            // Convert `if let` to a match statement internally
            // if let Pattern = expr { then } else { else_block }
            // becomes: match expr { Pattern => { then }, _ => { else_block } }
            let then_body = self.alloc_expr(Expression::Block {
                statements: then_block,
                location: self.current_location(),
            });

            let mut arms = vec![MatchArm {
                pattern,
                guard: None,
                body: then_body,
            }];

            // Add wildcard arm for else block (or empty block if no else)
            // This ensures exhaustive pattern matching in Rust
            let else_body = if let Some(else_stmts) = else_block {
                self.alloc_expr(Expression::Block {
                    statements: else_stmts,
                    location: self.current_location(),
                })
            } else {
                self.alloc_expr(Expression::Block {
                    statements: vec![],
                    location: self.current_location(),
                }) // Empty block if no else clause
            };

            arms.push(MatchArm {
                pattern: Pattern::Wildcard,
                guard: None,
                body: else_body,
            });

            Ok(self.alloc_stmt(Statement::Match {
                value,
                arms,
                location: self.current_location(),
            }))
        } else {
            // Regular if statement
            let condition = self.parse_expression()?;

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

            Ok(self.alloc_stmt(Statement::If {
                condition,
                then_block,
                else_block,
                location: self.current_location(),
            }))
        }
    }

    fn parse_match(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::Match)?;

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

                if is_assignment {
                    // Parse as statement (assignment)
                    let stmt = self.parse_statement()?;
                    // Wrap in block expression
                    let block = self.alloc_expr(Expression::Block {
                        statements: vec![stmt],
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

        Ok(self.alloc_stmt(Statement::Match {
            value,
            arms,
            location: self.current_location(),
        }))
    }

    // ========================================================================
    // SECTION 6: PATTERN PARSING
    // ========================================================================

    fn parse_loop(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::Loop)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(self.alloc_stmt(Statement::Loop {
            body,
            location: self.current_location(),
        }))
    }

    fn parse_while(&mut self) -> Result<&'static Statement<'static>, String> {
        self.expect(Token::While)?;

        // Check for `while let` pattern
        if self.peek(0) == Some(&Token::Let) {
            self.advance(); // consume 'let'

            // Parse pattern
            let pattern = self.parse_pattern()?;

            self.expect(Token::Assign)?; // '='

            // Parse the expression to match against
            let expr = self.parse_expression()?;

            self.expect(Token::LBrace)?;
            let body = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;

            // Desugar `while let` into a loop with match
            // while let pattern = expr { body }
            // becomes:
            // loop {
            //     match expr {
            //         pattern => { body }
            //         _ => break
            //     }
            // }
            let body_block = self.alloc_expr(Expression::Block {
                statements: body.clone(),
                location: self.current_location(),
            });

            let break_stmt = self.alloc_stmt(Statement::Break {
                location: self.current_location(),
            });

            let break_block = self.alloc_expr(Expression::Block {
                statements: vec![break_stmt],
                location: self.current_location(),
            });

            let match_stmt = self.alloc_stmt(Statement::Match {
                value: expr,
                arms: vec![
                    MatchArm {
                        pattern,
                        guard: None,
                        body: body_block,
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: break_block,
                    },
                ],
                location: self.current_location(),
            });

            Ok(self.alloc_stmt(Statement::Loop {
                body: vec![match_stmt],
                location: self.current_location(),
            }))
        } else {
            // Regular while loop
            let condition = self.parse_expression()?;

            self.expect(Token::LBrace)?;
            let body = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;

            Ok(self.alloc_stmt(Statement::While {
                condition,
                body,
                location: self.current_location(),
            }))
        }
    }
}
