// Pattern Parser - Windjammer Pattern Parsing Functions
//
// This module contains functions for parsing patterns in Windjammer.
// Patterns are used in let statements, match arms, function parameters, and for loops.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    /// Parse a pattern with OR support: pattern1 | pattern2 | pattern3
    pub fn parse_pattern_with_or(&mut self) -> Result<Pattern, String> {
        let first = self.parse_pattern()?;

        // Check for OR patterns: pattern1 | pattern2
        if self.current_token() == &Token::Pipe {
            let mut patterns = vec![first];

            while self.current_token() == &Token::Pipe {
                self.advance();
                patterns.push(self.parse_pattern()?);
            }

            Ok(Pattern::Or(patterns))
        } else {
            Ok(first)
        }
    }

    /// Parse a single pattern
    pub fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match self.current_token() {
            Token::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::LParen => {
                // Tuple pattern
                self.advance();
                let mut patterns = Vec::new();

                while self.current_token() != &Token::RParen {
                    patterns.push(self.parse_pattern()?);

                    if self.current_token() == &Token::Comma {
                        self.advance();
                    } else {
                        break; // No comma, must be end of tuple
                    }
                }

                self.expect(Token::RParen)?;
                Ok(Pattern::Tuple(patterns))
            }
            Token::BoolLiteral(b) => {
                let b = *b;
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(b)))
            }
            Token::IntLiteral(n) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Int(n)))
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            Token::CharLiteral(c) => {
                let c = *c;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            Token::Ident(name) => {
                let mut qualified_path = name.clone();
                self.advance();

                // Check if it's a qualified enum variant: Result.Ok(x) or ClientMessage::Ping
                // or module::Type::Variant (multi-level path)
                if self.current_token() == &Token::Dot || self.current_token() == &Token::ColonColon
                {
                    // Build the full qualified path (could be multiple segments)
                    // e.g., physics::Collider2D::Box or std::option::Option::Some
                    loop {
                        let separator = if self.current_token() == &Token::Dot {
                            "."
                        } else if self.current_token() == &Token::ColonColon {
                            "::"
                        } else {
                            break;
                        };
                        self.advance();

                        // Get next segment - must be an identifier
                        if let Token::Ident(segment) = self.current_token() {
                            qualified_path.push_str(separator);
                            qualified_path.push_str(segment);
                            self.advance();
                        } else {
                            return Err(format!(
                                "Expected identifier after {}, got {:?}",
                                separator,
                                self.current_token()
                            ));
                        }

                        // Check if there's another separator (more path segments)
                        // or if we've reached the variant (followed by { or ( or nothing)
                        if self.current_token() != &Token::Dot 
                            && self.current_token() != &Token::ColonColon {
                            break;
                        }
                    }

                    // Check for binding: Result.Ok(x) or Result.Ok(_) or Result.Ok(true)
                    let binding = if self.current_token() == &Token::LParen {
                        self.advance();
                        let b = match self.current_token() {
                            Token::Underscore => {
                                self.advance();
                                EnumPatternBinding::Wildcard
                            }
                            Token::Ident(name) => {
                                let name = name.clone();
                                self.advance();
                                EnumPatternBinding::Named(name)
                            }
                            Token::BoolLiteral(_)
                            | Token::IntLiteral(_)
                            | Token::StringLiteral(_)
                            | Token::CharLiteral(_) => {
                                // Literal pattern inside enum variant: Ok(true), Err(404), etc.
                                // Parse the literal pattern recursively
                                let _pattern = self.parse_pattern()?;
                                // For now, treat literal patterns as wildcards in enum bindings
                                // TODO: Properly support literal patterns in enum variants
                                EnumPatternBinding::Wildcard
                            }
                            Token::LBrace => {
                                // Struct-like enum variant: Variant { field1, field2 }
                                // For now, just consume the whole thing and treat as wildcard
                                // TODO: Properly parse struct patterns
                                let mut depth = 1;
                                self.advance(); // consume {
                                while depth > 0 && self.current_token() != &Token::Eof {
                                    match self.current_token() {
                                        Token::LBrace => depth += 1,
                                        Token::RBrace => depth -= 1,
                                        _ => {}
                                    }
                                    self.advance();
                                }
                                EnumPatternBinding::Wildcard
                            }
                            _ => EnumPatternBinding::None,
                        };
                        if self.current_token() == &Token::RParen {
                            self.expect(Token::RParen)?;
                        }
                        b
                    } else if self.current_token() == &Token::LBrace {
                        // Struct-like enum variant without parens: Variant { field1, field2 }
                        let mut depth = 1;
                        self.advance(); // consume {
                        while depth > 0 && self.current_token() != &Token::Eof {
                            match self.current_token() {
                                Token::LBrace => depth += 1,
                                Token::RBrace => depth -= 1,
                                _ => {}
                            }
                            self.advance();
                        }
                        EnumPatternBinding::Wildcard
                    } else {
                        EnumPatternBinding::None
                    };

                    Ok(Pattern::EnumVariant(
                        qualified_path,
                        binding,
                    ))
                } else if self.current_token() == &Token::LParen {
                    // Unqualified enum variant with parameter: Some(x), Ok(value), Err(e), Some(_), Ok((a, b))
                    self.advance();

                    // Handle underscore (Some(_)), identifier (Some(x)), mut identifier (Some(mut x)), or nested patterns (Ok((a, b)))
                    let binding = match self.current_token() {
                        Token::Underscore => {
                            self.advance();
                            EnumPatternBinding::Wildcard
                        }
                        Token::Mut => {
                            // mut binding: Some(mut x)
                            self.advance();
                            if let Token::Ident(b) = self.current_token() {
                                let b = b.clone();
                                self.advance();
                                // For now, treat mut bindings same as regular bindings
                                // The Rust codegen will handle the mut keyword
                                EnumPatternBinding::Named(format!("mut {}", b))
                            } else {
                                return Err(format!("Expected identifier after mut in enum pattern (at token position {})", self.position));
                            }
                        }
                        Token::Ident(b) => {
                            let b = b.clone();
                            self.advance();
                            EnumPatternBinding::Named(b)
                        }
                        Token::LParen => {
                            // Nested pattern like Ok((a, b))
                            // Parse as a tuple pattern and convert to string representation
                            let nested_pattern = self.parse_pattern()?;
                            // Convert the pattern to a string for now
                            // TODO: Extend EnumPatternBinding to support nested patterns
                            EnumPatternBinding::Named(Self::pattern_to_string(&nested_pattern))
                        }
                        Token::BoolLiteral(_)
                        | Token::IntLiteral(_)
                        | Token::StringLiteral(_)
                        | Token::CharLiteral(_) => {
                            // Literal pattern inside enum variant: Ok(true), Err(404), etc.
                            // Parse the literal pattern and convert to string
                            let pattern = self.parse_pattern()?;
                            EnumPatternBinding::Named(Self::pattern_to_string(&pattern))
                        }
                        _ => {
                            return Err(format!(
                                "Expected binding name or _ in enum pattern (at token position {})",
                                self.position
                            ));
                        }
                    };

                    self.expect(Token::RParen)?;
                    Ok(Pattern::EnumVariant(qualified_path, binding))
                } else {
                    // Check if this could be an enum variant without parameters (None, Empty, etc.)
                    // For now, treat as identifier - the analyzer will determine if it's an enum variant
                    Ok(Pattern::Identifier(qualified_path))
                }
            }
            _ => Err(format!("Expected pattern, got {:?}", self.current_token())),
        }
    }

    /// Helper: Extract a simple name from a pattern for use in generated code
    pub fn pattern_to_name(pattern: &Pattern) -> String {
        match pattern {
            Pattern::Identifier(name) => name.clone(),
            Pattern::Reference(inner) => {
                // For reference patterns, use the inner pattern's name
                Self::pattern_to_name(inner)
            }
            Pattern::Tuple(patterns) => {
                // For tuple patterns, generate a name like "_tuple_param"
                format!("_tuple_{}", patterns.len())
            }
            Pattern::EnumVariant(name, _) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(_) => "_lit".to_string(),
            Pattern::Or(patterns) => {
                // Use the first pattern's name
                if let Some(first) = patterns.first() {
                    Self::pattern_to_name(first)
                } else {
                    "_or_pattern".to_string()
                }
            }
        }
    }

    /// Helper: Convert a pattern to a string representation for enum bindings
    pub fn pattern_to_string(pattern: &Pattern) -> String {
        match pattern {
            Pattern::Identifier(name) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::Tuple(patterns) => {
                let parts: Vec<String> = patterns.iter().map(Self::pattern_to_string).collect();
                format!("({})", parts.join(", "))
            }
            Pattern::Reference(inner) => format!("&{}", Self::pattern_to_string(inner)),
            Pattern::EnumVariant(name, binding) => match binding {
                EnumPatternBinding::None => name.clone(),
                EnumPatternBinding::Named(b) => format!("{}({})", name, b),
                EnumPatternBinding::Wildcard => format!("{}(_)", name),
            },
            Pattern::Literal(lit) => format!("{:?}", lit),
            Pattern::Or(patterns) => {
                let parts: Vec<String> = patterns.iter().map(Self::pattern_to_string).collect();
                parts.join(" | ")
            }
        }
    }
}
