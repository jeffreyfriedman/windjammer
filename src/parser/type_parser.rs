// Type Parser - Windjammer Type Parsing Functions
//
// This module contains functions for parsing type annotations in Windjammer.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    /// Convert a Type to a string representation (for error messages and debugging)
    #[allow(clippy::only_used_in_recursion)]
    pub fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Uint => "uint".to_string(),
            Type::Float => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Custom(name) => name.clone(),
            Type::Generic(name) => name.clone(),
            Type::Reference(inner) => format!("&{}", self.type_to_string(inner)),
            Type::MutableReference(inner) => format!("&mut {}", self.type_to_string(inner)),
            Type::RawPointer { mutable, pointee } => {
                if *mutable {
                    format!("*mut {}", self.type_to_string(pointee))
                } else {
                    format!("*const {}", self.type_to_string(pointee))
                }
            }
            Type::Option(inner) => format!("Option<{}>", self.type_to_string(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                self.type_to_string(ok),
                self.type_to_string(err)
            ),
            Type::Vec(inner) => format!("Vec<{}>", self.type_to_string(inner)),
            Type::Array(inner, size) => format!("[{}; {}]", self.type_to_string(inner), size),
            Type::Tuple(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| self.type_to_string(t)).collect();
                format!("({})", type_strs.join(", "))
            }
            Type::Parameterized(base, args) => {
                let arg_strs: Vec<String> = args.iter().map(|t| self.type_to_string(t)).collect();
                format!("{}<{}>", base, arg_strs.join(", "))
            }
            Type::Associated(base, name) => format!("{}::{}", base, name),
            Type::TraitObject(trait_name) => format!("dyn {}", trait_name),
            Type::ImplTrait(trait_name) => format!("trait {}", trait_name),
            Type::Infer => "_".to_string(),
            Type::FunctionPointer {
                params,
                return_type,
            } => {
                let param_strs: Vec<String> =
                    params.iter().map(|t| self.type_to_string(t)).collect();
                if let Some(ret) = return_type {
                    format!(
                        "fn({}) -> {}",
                        param_strs.join(", "),
                        self.type_to_string(ret)
                    )
                } else {
                    format!("fn({})", param_strs.join(", "))
                }
            }
        }
    }

    /// Parse generic type parameters: <T, U: Display, V: Clone + Send>
    pub fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        let mut type_params = Vec::new();
        if self.current_token() == &Token::Lt {
            self.advance(); // Consume <
            while self.current_token() != &Token::Gt {
                let name = if let Token::Ident(n) = self.current_token() {
                    let name = n.clone();
                    self.advance();
                    name
                } else {
                    return Err("Expected type parameter name".to_string());
                };

                let mut bounds = Vec::new();
                if self.current_token() == &Token::Colon {
                    self.advance(); // Consume :
                    while self.current_token() != &Token::Comma
                        && self.current_token() != &Token::Gt
                    {
                        if let Token::Ident(bound) = self.current_token() {
                            bounds.push(bound.clone());
                            self.advance();
                        } else {
                            return Err("Expected trait bound name".to_string());
                        }

                        if self.current_token() == &Token::Plus {
                            self.advance(); // Consume + for multiple bounds
                        } else {
                            break;
                        }
                    }
                }

                type_params.push(TypeParam { name, bounds });

                if self.current_token() == &Token::Comma {
                    self.advance(); // Consume ,
                } else {
                    break;
                }
            }
            self.expect_gt_or_split_shr()?; // Consume > (handles >> for nested generics)
        }
        Ok(type_params)
    }

    /// Parse where clause: where T: Display, U: Clone + Send, P::Output: Debug
    pub fn parse_where_clause(&mut self) -> Result<Vec<(String, Vec<String>)>, String> {
        let mut where_clause = Vec::new();
        if self.current_token() == &Token::Where {
            self.advance(); // Consume where
            while self.current_token() != &Token::LBrace
                && self.current_token() != &Token::Semicolon
            {
                // Parse type parameter name (can be T or P::Output for associated types)
                let type_param = if let Token::Ident(n) = self.current_token() {
                    let mut name = n.clone();
                    self.advance();

                    // Check for associated type syntax: P::Output
                    while self.current_token() == &Token::ColonColon {
                        self.advance(); // Consume ::
                        if let Token::Ident(assoc_name) = self.current_token() {
                            name.push_str("::");
                            name.push_str(assoc_name);
                            self.advance();
                        } else {
                            return Err("Expected associated type name after '::'".to_string());
                        }
                    }
                    name
                } else {
                    return Err("Expected type parameter name in where clause".to_string());
                };

                // Expect :
                if self.current_token() != &Token::Colon {
                    return Err("Expected ':' after type parameter in where clause".to_string());
                }
                self.advance();

                // Parse trait bounds
                let mut bounds = Vec::new();
                loop {
                    if let Token::Ident(bound) = self.current_token() {
                        bounds.push(bound.clone());
                        self.advance();
                    } else {
                        return Err("Expected trait bound in where clause".to_string());
                    }

                    if self.current_token() == &Token::Plus {
                        self.advance(); // Consume + for multiple bounds
                    } else {
                        break;
                    }
                }

                where_clause.push((type_param, bounds));

                if self.current_token() == &Token::Comma {
                    self.advance(); // Consume ,
                } else {
                    break;
                }
            }
        }
        Ok(where_clause)
    }

    /// Parse a type annotation
    pub fn parse_type(&mut self) -> Result<Type, String> {
        // Handle raw pointer types: *const T, *mut T
        if self.current_token() == &Token::Star {
            self.advance();
            
            // Check for const or mut
            let mutable = if self.current_token() == &Token::Const {
                self.advance();
                false
            } else if self.current_token() == &Token::Mut {
                self.advance();
                true
            } else {
                return Err("Expected 'const' or 'mut' after '*' in pointer type".to_string());
            };
            
            let pointee = Box::new(self.parse_type()?);
            return Ok(Type::RawPointer { mutable, pointee });
        }
        
        // Handle reference types
        if self.current_token() == &Token::Ampersand {
            self.advance();
            if self.current_token() == &Token::Mut {
                self.advance();
                let inner = Box::new(self.parse_type()?);
                return Ok(Type::MutableReference(inner));
            } else {
                let inner = Box::new(self.parse_type()?);
                return Ok(Type::Reference(inner));
            }
        }

        let base_type = match self.current_token() {
            Token::Dyn => {
                // Parse: dyn TraitName
                self.advance();
                if let Token::Ident(trait_name) = self.current_token() {
                    let name = trait_name.clone();
                    self.advance();
                    Type::TraitObject(name)
                } else {
                    return Err("Expected trait name after 'dyn'".to_string());
                }
            }
            Token::Trait => {
                // Parse: trait TraitName (Windjammer syntax - compiler infers dispatch)
                self.advance();
                if let Token::Ident(trait_name) = self.current_token() {
                    let name = trait_name.clone();
                    self.advance();
                    Type::ImplTrait(name)
                } else {
                    return Err("Expected trait name after 'trait'".to_string());
                }
            }
            Token::Int => {
                self.advance();
                Type::Int
            }
            Token::Int32 => {
                self.advance();
                Type::Int32
            }
            Token::Uint => {
                self.advance();
                Type::Uint
            }
            Token::Float => {
                self.advance();
                Type::Float
            }
            Token::Bool => {
                self.advance();
                Type::Bool
            }
            Token::String => {
                self.advance();
                Type::String
            }
            Token::LBracket => {
                // Array/Slice type: [T] or fixed-size array: [T; N]
                self.advance();
                let inner = Box::new(self.parse_type()?);

                // Check for fixed-size array syntax: [T; N]
                if self.current_token() == &Token::Semicolon {
                    self.advance();

                    // Parse the size - must be a literal integer
                    let size = match self.current_token() {
                        Token::IntLiteral(n) => {
                            let size = *n as usize;
                            self.advance();
                            size
                        }
                        _ => {
                            return Err(format!(
                                "Expected integer literal for array size, got {:?}",
                                self.current_token()
                            ));
                        }
                    };

                    self.expect(Token::RBracket)?;
                    Type::Array(inner, size)
                } else {
                    self.expect(Token::RBracket)?;
                    // [T] without size is a dynamic array (Vec)
                    Type::Vec(inner)
                }
            }
            Token::Fn => {
                // Function pointer type: fn(int, string) -> bool
                self.advance(); // consume 'fn'
                self.expect(Token::LParen)?;

                let mut params = Vec::new();
                while self.current_token() != &Token::RParen {
                    params.push(self.parse_type()?);

                    if self.current_token() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(Token::RParen)?;

                let return_type = if self.current_token() == &Token::Arrow {
                    self.advance();
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };

                Type::FunctionPointer {
                    params,
                    return_type,
                }
            }
            Token::LParen => {
                // Tuple type: (T1, T2, T3) or unit type: ()
                self.advance();

                // Check for unit type ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    return Ok(Type::Tuple(vec![])); // Unit type is an empty tuple
                }

                let mut types = Vec::new();

                while self.current_token() != &Token::RParen {
                    types.push(self.parse_type()?);

                    if self.current_token() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }

                self.expect(Token::RParen)?;
                Type::Tuple(types)
            }
            Token::Ident(name) => {
                let mut type_name = name.clone();
                self.advance();

                // Handle qualified type names with both . and :: (module.Type or module::Type)
                loop {
                    if self.current_token() == &Token::Dot {
                        self.advance();
                        if let Token::Ident(segment) = self.current_token() {
                            type_name.push('.');
                            type_name.push_str(segment);
                            self.advance();
                        } else {
                            return Err("Expected identifier after '.' in type name".to_string());
                        }
                    } else if self.current_token() == &Token::ColonColon {
                        // Look ahead to check if this is an associated type or path segment
                        if self.position + 1 < self.tokens.len() {
                            // Allow keywords as identifiers in type paths (e.g., std::thread::JoinHandle)
                            let next_segment_opt = match &self.tokens[self.position + 1].token {
                                Token::Ident(n) => Some(n.clone()),
                                Token::Thread => Some("thread".to_string()),
                                Token::Async => Some("async".to_string()),
                                _ => None,
                            };

                            if let Some(next_segment_str) = next_segment_opt {
                                // Could be either:
                                // 1. Path segment: std::fs::File
                                // 2. Associated type: Self::Item

                                // For now, check if the next token after the identifier is a generic or end
                                // to determine if this is the final segment (associated type)
                                if self.position + 2 < self.tokens.len() {
                                    let after_next = &self.tokens[self.position + 2];
                                    match &after_next.token {
                                        Token::Lt => {
                                            // This is a parameterized type (e.g., HashMap<K, V>)
                                            // Add to path and break to let generic parsing handle it
                                            type_name.push_str("::");
                                            type_name.push_str(&next_segment_str);
                                            self.advance(); // consume ::
                                            self.advance(); // consume identifier
                                            break; // Exit loop to handle generics
                                        }
                                        Token::Comma
                                        | Token::Gt
                                        | Token::RParen
                                        | Token::RBrace
                                        | Token::Semicolon
                                        | Token::FatArrow
                                        | Token::LBrace
                                        | Token::Where => {
                                            // This looks like an associated type (final segment)
                                            self.advance(); // consume ::
                                            self.advance(); // consume identifier
                                            return Ok(Type::Associated(
                                                type_name,
                                                next_segment_str,
                                            ));
                                        }
                                        Token::ColonColon => {
                                            // More path segments to come
                                            type_name.push_str("::");
                                            type_name.push_str(&next_segment_str);
                                            self.advance(); // consume ::
                                            self.advance(); // consume identifier
                                            continue;
                                        }
                                        _ => {
                                            // Assume associated type
                                            self.advance(); // consume ::
                                            self.advance(); // consume identifier
                                            return Ok(Type::Associated(
                                                type_name,
                                                next_segment_str,
                                            ));
                                        }
                                    }
                                } else {
                                    // End of tokens, treat as associated type
                                    self.advance(); // consume ::
                                    self.advance(); // consume identifier
                                    return Ok(Type::Associated(type_name, next_segment_str));
                                }
                            } else {
                                return Err(
                                    "Expected identifier after '::' in type name".to_string()
                                );
                            }
                        } else {
                            return Err("Expected identifier after '::' in type name".to_string());
                        }
                    } else {
                        break;
                    }
                }

                // Check for generic parameters
                // BUT: Primitive types like usize, i32, u32, etc. can't have generics
                // If we see `<` after a primitive type, it's a comparison operator, not generics!
                let is_primitive_type = matches!(
                    type_name.as_str(),
                    "usize" | "isize"
                    | "u8" | "u16" | "u32" | "u64" | "u128"
                    | "i8" | "i16" | "i32" | "i64" | "i128"
                    | "f32" | "f64"
                    | "char" | "str" | "bool"
                    | "unit" | "()"
                );
                
                if !is_primitive_type && self.current_token() == &Token::Lt {
                    self.advance();

                    // Handle Vec<T>, Option<T>, Result<T, E>
                    if type_name == "Vec" {
                        let inner = Box::new(self.parse_type()?);
                        self.expect_gt_or_split_shr()?;
                        Type::Vec(inner)
                    } else if type_name == "Option" {
                        let inner = Box::new(self.parse_type()?);
                        self.expect_gt_or_split_shr()?;
                        Type::Option(inner)
                    } else if type_name == "Result" {
                        let ok_type = Box::new(self.parse_type()?);
                        self.expect(Token::Comma)?;
                        let err_type = Box::new(self.parse_type()?);
                        self.expect_gt_or_split_shr()?;
                        Type::Result(ok_type, err_type)
                    } else {
                        // Generic custom type: Box<T>, HashMap<K, V>, etc.
                        let mut type_args = Vec::new();

                        loop {
                            type_args.push(self.parse_type()?);

                            if self.current_token() == &Token::Comma {
                                self.advance();
                            } else if self.current_token() == &Token::Gt
                                || self.current_token() == &Token::Shr
                            {
                                // Handle both > and >> (for nested generics like HashMap<K, Vec<V>>)
                                self.expect_gt_or_split_shr()?;
                                break;
                            } else {
                                eprintln!("DEBUG: Parsing type arguments for: {}", type_name);
                                eprintln!("DEBUG: After parsing type arg, current token: {:?} at position: {}", self.current_token(), self.position);
                                return Err(format!(
                                    "Expected ',' or '>' in type arguments for '{}', got {:?} at position {}",
                                    type_name, self.current_token(), self.position
                                ));
                            }
                        }

                        Type::Parameterized(type_name, type_args)
                    }
                } else {
                    Type::Custom(type_name)
                }
            }
            Token::Underscore => {
                // Type inference placeholder: _
                self.advance();
                Type::Infer
            }
            Token::Self_ => {
                // Self type (e.g. in &self, &mut self parameter types)
                self.advance();
                Type::Custom("Self".to_string())
            }
            _ => return Err(format!("Expected type, got {:?}", self.current_token())),
        };

        Ok(base_type)
    }

    /// Public wrapper for parse_type (for external use)
    pub fn parse_type_public(&mut self) -> Result<Type, String> {
        self.parse_type()
    }
}
