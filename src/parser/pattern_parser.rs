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

                    // Check for binding: Result.Ok(x) or Result.Ok(_) or Rgb(r, g, b)
                    let binding = if self.current_token() == &Token::LParen {
                        self.advance();
                        
                        // Parse patterns separated by commas
                        let mut patterns = Vec::new();
                        
                        // Handle empty parens: Variant()
                        if self.current_token() == &Token::RParen {
                            self.advance();
                            return Ok(Pattern::EnumVariant(qualified_path, EnumPatternBinding::None));
                        }
                        
                        // Parse first pattern
                        patterns.push(self.parse_pattern()?);
                        
                        // Check if there are more patterns (comma-separated)
                        while self.current_token() == &Token::Comma {
                            self.advance();
                            
                            // Allow trailing comma
                            if self.current_token() == &Token::RParen {
                                break;
                            }
                            
                            patterns.push(self.parse_pattern()?);
                        }
                        
                        self.expect(Token::RParen)?;
                        
                        // Determine binding type based on number of patterns
                        if patterns.len() == 1 {
                            // Single pattern: check what it is
                            match &patterns[0] {
                                Pattern::Wildcard => EnumPatternBinding::Wildcard,
                                Pattern::Identifier(name) => EnumPatternBinding::Single(name.clone()),
                                _ => EnumPatternBinding::Tuple(patterns),
                            }
                        } else {
                            // Multiple patterns: tuple binding
                            EnumPatternBinding::Tuple(patterns)
                        }
                    } else if self.current_token() == &Token::LBrace {
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
                    // Unqualified enum variant with parameter(s): Some(x), Rgb(r, g, b)
                    self.advance();

                    // Parse patterns separated by commas
                    let mut patterns = Vec::new();
                    
                    // Handle empty parens: Variant()
                    if self.current_token() == &Token::RParen {
                        self.advance();
                        return Ok(Pattern::EnumVariant(qualified_path, EnumPatternBinding::None));
                    }
                    
                    // Parse first pattern
                    patterns.push(self.parse_pattern()?);
                    
                    // Check if there are more patterns (comma-separated)
                    while self.current_token() == &Token::Comma {
                        self.advance();
                        
                        // Allow trailing comma
                        if self.current_token() == &Token::RParen {
                            break;
                        }
                        
                        patterns.push(self.parse_pattern()?);
                    }
                    
                    self.expect(Token::RParen)?;
                    
                    // Determine binding type based on number of patterns
                    let binding = if patterns.len() == 1 {
                        // Single pattern: check what it is
                        match &patterns[0] {
                            Pattern::Wildcard => EnumPatternBinding::Wildcard,
                            Pattern::Identifier(name) => EnumPatternBinding::Single(name.clone()),
                            _ => EnumPatternBinding::Tuple(patterns),
                        }
                    } else {
                        // Multiple patterns: tuple binding
                        EnumPatternBinding::Tuple(patterns)
                    };

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
                EnumPatternBinding::Single(b) => format!("{}({})", name, b),
                EnumPatternBinding::Wildcard => format!("{}(_)", name),
                EnumPatternBinding::Tuple(patterns) => {
                    let parts: Vec<String> = patterns.iter().map(Self::pattern_to_string).collect();
                    format!("{}({})", name, parts.join(", "))
                }
            },
            Pattern::Literal(lit) => format!("{:?}", lit),
            Pattern::Or(patterns) => {
                let parts: Vec<String> = patterns.iter().map(Self::pattern_to_string).collect();
                parts.join(" | ")
            }
        }
    }

    /// Check if a pattern is refutable (can fail to match).
    /// 
    /// Irrefutable patterns (always match):
    /// - Identifier: `x`, `_`
    /// - Tuple: `(a, b)` (if all elements are irrefutable)
    /// - Reference: `&x` (if inner is irrefutable)
    /// 
    /// Refutable patterns (can fail):
    /// - Enum variant: `Some(x)`, `Ok(value)`
    /// - Literal: `42`, `"hello"`, `true`
    /// - Or pattern: `x | y`
    pub fn is_pattern_refutable(pattern: &Pattern) -> bool {
        match pattern {
            // Irrefutable patterns
            Pattern::Wildcard => false,
            Pattern::Identifier(_) => false,
            Pattern::Tuple(patterns) => {
                // Tuple is refutable if any element is refutable
                patterns.iter().any(Self::is_pattern_refutable)
            }
            Pattern::Reference(inner) => Self::is_pattern_refutable(inner),
            
            // Refutable patterns
            Pattern::EnumVariant(_, _) => true,
            Pattern::Literal(_) => true,
            Pattern::Or(_) => true,
        }
    }
}
