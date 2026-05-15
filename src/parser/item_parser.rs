// Item Parser - Windjammer Top-Level Item Parsing Functions
//
// This module contains functions for parsing top-level items in Windjammer.
// Items include functions, structs, enums, traits, impl blocks, and their components.
//
// Struct/enum/function/trait bodies live in `struct_parser`, `enum_parser`,
// `function_parser`, and `trait_parser`.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    /// Collect all consecutive doc comments into a single string
    pub(in crate::parser) fn collect_doc_comments(&mut self) -> Option<String> {
        let mut doc_comments = Vec::new();
        while let Token::DocComment(comment) = self.current_token() {
            doc_comments.push(comment.clone());
            self.advance();
        }

        if doc_comments.is_empty() {
            None
        } else {
            Some(doc_comments.join("\n"))
        }
    }

    pub(crate) fn parse_impl(
        &mut self,
        is_extern_block: bool,
    ) -> Result<ImplBlock<'static>, String> {
        // Parse: impl<T> Type { } or impl Trait for Type { } or impl Trait<TypeArgs> for Type { }

        // Parse type parameters: impl<T, U> Box<T, U> { ... }
        let type_params = self.parse_type_params()?;

        let first_name = if let Token::Ident(name) = self.current_token() {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err("Expected type or trait name after impl".to_string());
        };

        // Handle parameterized name: Box<T>, From<int>, etc.
        // This could be either:
        // 1. impl Box<T> { ... } - inherent impl on generic type
        // 2. impl From<int> for String - trait impl with type args
        let (first_name_with_args, first_type_args) = if self.current_token() == &Token::Lt {
            self.advance();
            let mut type_args = Vec::new();

            loop {
                // Parse a full type to handle both generic params (T) and concrete types (int)
                let type_arg = self.parse_type()?;
                type_args.push(type_arg);

                if self.current_token() == &Token::Comma {
                    self.advance();
                } else if self.current_token() == &Token::Gt {
                    self.advance();
                    break;
                } else {
                    return Err("Expected ',' or '>' in impl type parameters".to_string());
                }
            }

            // For the full name with args (used if this is the type_name)
            let args_str = type_args
                .iter()
                .map(|t| self.type_to_string(t))
                .collect::<Vec<_>>()
                .join(", ");
            (format!("{}<{}>", first_name, args_str), Some(type_args))
        } else {
            (first_name.clone(), None)
        };

        // Check if this is "impl Trait for Type" or just "impl Type"
        let (trait_name, trait_type_args, type_name): (_, _, String) =
            if self.current_token() == &Token::For {
                self.advance(); // consume "for"

                // Parse the type name after 'for' - could be primitive (int, string) or custom (MyType)
                let base_type_name = match self.current_token() {
                    Token::Ident(name) => {
                        let n = name.clone();
                        self.advance();
                        n
                    }
                    Token::Int => {
                        self.advance();
                        "int".to_string()
                    }
                    Token::Int32 => {
                        self.advance();
                        "i32".to_string()
                    }
                    Token::Uint => {
                        self.advance();
                        "uint".to_string()
                    }
                    Token::Float => {
                        self.advance();
                        "float".to_string()
                    }
                    Token::Bool => {
                        self.advance();
                        "bool".to_string()
                    }
                    Token::String => {
                        self.advance();
                        "string".to_string()
                    }
                    _ => {
                        return Err("Expected type name after 'for'".to_string());
                    }
                };

                let mut type_name = base_type_name;

                // Handle parameterized type name after 'for': impl<T> Trait for Box<T>
                if self.current_token() == &Token::Lt {
                    type_name.push('<');
                    self.advance();

                    loop {
                        // Parse full type to handle both generic params and concrete types
                        let type_arg = self.parse_type()?;
                        type_name.push_str(&self.type_to_string(&type_arg));

                        if self.current_token() == &Token::Comma {
                            type_name.push_str(", ");
                            self.advance();
                        } else if self.current_token() == &Token::Gt {
                            type_name.push('>');
                            self.advance();
                            break;
                        } else {
                            return Err(
                                "Expected ',' or '>' in type parameters after 'for'".to_string()
                            );
                        }
                    }
                }

                (Some(first_name), first_type_args, type_name)
            } else {
                (None, None, first_name_with_args)
            };

        // Parse where clause (optional): where T: Clone, U: Debug
        let where_clause = self.parse_where_clause()?;

        self.expect(Token::LBrace)?;

        let mut associated_types = Vec::new();
        let mut functions = Vec::new();

        while self.current_token() != &Token::RBrace {
            // Check if this is an associated type implementation: type Name = Type;
            if self.current_token() == &Token::Type {
                self.advance(); // consume 'type'

                let assoc_name = if let Token::Ident(n) = self.current_token() {
                    let name = n.clone();
                    self.advance();
                    name
                } else {
                    return Err("Expected associated type name in impl".to_string());
                };

                self.expect(Token::Assign)?;

                let concrete_type = self.parse_type()?;

                // Semicolons are optional for associated types (like Swift, Kotlin, Go)
                if self.current_token() == &Token::Semicolon {
                    self.advance(); // consume optional semicolon
                }

                associated_types.push(AssociatedType {
                    name: assoc_name,
                    concrete_type: Some(concrete_type),
                });

                continue;
            }

            // Capture all consecutive doc comments (/// or //!)
            let doc_comment = self.collect_doc_comments();

            // Skip decorators for now (could be added later)
            let mut decorators = Vec::new();
            while let Token::Decorator(_) = self.current_token() {
                decorators.push(self.parse_decorator()?);
            }

            // Parse function (pub optional)
            let is_pub = if self.current_token() == &Token::Pub {
                self.advance();
                true
            } else {
                false
            };

            let is_async = if self.current_token() == &Token::Async {
                self.advance();
                true
            } else {
                false
            };

            self.expect(Token::Fn)?;
            let mut func = self.parse_function()?;
            func.is_extern = is_extern_block;
            func.is_pub = is_pub;
            func.is_async = is_async;
            func.decorators = decorators;
            func.doc_comment = doc_comment; // Doc comment from before the method
            func.parent_type = Some(type_name.clone()); // Track which impl block this function belongs to
            func.impl_trait = trait_name.clone();
            functions.push(func);
        }

        self.expect(Token::RBrace)?;

        Ok(ImplBlock {
            type_name,
            type_params,
            where_clause,
            trait_name,
            trait_type_args,
            associated_types,
            functions,
            decorators: Vec::new(),
            is_extern: is_extern_block,
        })
    }

    pub(crate) fn parse_decorator(&mut self) -> Result<Decorator<'static>, String> {
        if let Token::Decorator(name) = self.current_token() {
            let name = name.clone();
            self.advance();

            // Check for decorator arguments: @route("/path") or @cache(ttl: 60)
            let arguments = if self.current_token() == &Token::LParen {
                self.advance();
                self.parse_decorator_arguments()?
            } else {
                Vec::new()
            };

            Ok(Decorator { name, arguments })
        } else {
            Err("Expected decorator".to_string())
        }
    }

    fn parse_decorator_arguments(
        &mut self,
    ) -> Result<Vec<(String, &'static Expression<'static>)>, String> {
        let mut args = Vec::new();

        while self.current_token() != &Token::RParen {
            // Try to detect named arguments by lookahead
            let is_named_arg = if let Token::Ident(_) = self.current_token() {
                // Look ahead to see if there's : or = after the identifier
                if let Some(next_token) = self.peek(1) {
                    matches!(next_token, Token::Colon | Token::Assign)
                } else {
                    false
                }
            } else {
                false
            };

            if is_named_arg {
                // Named argument (key: value or key = value)
                if let Token::Ident(key) = self.current_token() {
                    let key = key.clone();
                    self.advance();
                    self.advance(); // consume : or =
                    let value = self.parse_expression()?;
                    args.push((key, value));
                } else {
                    unreachable!("is_named_arg should only be true for Ident");
                }
            } else {
                // Positional expression argument (parse full expression)
                let expr = self.parse_expression()?;
                args.push((String::new(), expr));
            }

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(Token::RParen)?;
        Ok(args)
    }

    pub(crate) fn parse_use(&mut self) -> Result<(Vec<String>, Option<String>), String> {
        // Note: Token::Use already consumed in parse_item

        let mut path = Vec::new();
        let mut path_str = String::new();

        // Handle relative imports: ./module or ../module
        if self.current_token() == &Token::Dot {
            path_str.push('.');
            self.advance();

            // Check for ./ or ../
            if self.current_token() == &Token::Slash {
                path_str.push('/');
                self.advance();
            } else if self.current_token() == &Token::Dot {
                // ../
                path_str.push('.');
                self.advance();
                if self.current_token() == &Token::Slash {
                    path_str.push('/');
                    self.advance();
                }
            }
        } else if self.current_token() == &Token::DotDot {
            // Handle .. token (lexer generates this for ..)
            path_str.push_str("..");
            self.advance();
            if self.current_token() == &Token::Slash {
                path_str.push('/');
                self.advance();
            }
        }

        // Parse the rest of the path (identifiers separated by :: or /)
        loop {
            // THE WINDJAMMER WAY: Support Rust-style path keywords (self, super, crate)
            // Allow keywords as identifiers in module paths
            let name_opt: Option<String> = match self.current_token() {
                Token::Ident(n) => Some(n.clone()), // Includes "super" and "crate" (not reserved)
                Token::Thread => Some("thread".to_string()),
                Token::Async => Some("async".to_string()),
                Token::Self_ => Some("self".to_string()), // "self" IS a reserved keyword
                _ => None,
            };

            if let Some(name) = name_opt {
                path_str.push_str(&name);
                self.advance();

                // Check for :: as separator (. and / are NOT supported - use :: for modules)
                if self.current_token() == &Token::ColonColon {
                    path_str.push_str("::");
                    self.advance();

                    // Check if next token is * (glob import)
                    if self.current_token() == &Token::Star {
                        path_str.push('*');
                        self.advance();
                        break;
                    }
                } else if self.current_token() == &Token::Slash {
                    // ERROR: / is not allowed for absolute module paths, use :: instead
                    // Note: / is still valid for relative imports like ./module or ../module
                    return Err("Use '::' for module paths, not '/'. Example: 'use std::fs' not 'use std/fs'".to_string());
                } else if self.current_token() == &Token::Dot {
                    // ERROR: . is not allowed for module paths, use :: instead
                    return Err("Use '::' for module paths, not '.'. Example: 'use std::fs' not 'use std.fs'".to_string());
                } else {
                    break;
                }
            } else if path_str.is_empty() {
                return Err("Expected identifier in use statement".to_string());
            } else {
                break;
            }
        }

        // Check for braced imports: use module::{A, B, C}
        if self.current_token() == &Token::LBrace {
            // Remove trailing :: if present (already added by previous iteration)
            if path_str.ends_with("::") {
                path_str.pop();
                path_str.pop();
            }

            self.advance(); // consume {
            path_str.push_str("::{");

            loop {
                if let Token::Ident(name) = self.current_token() {
                    path_str.push_str(name);
                    self.advance();

                    // Check for comma (more items) or closing brace
                    if self.current_token() == &Token::Comma {
                        path_str.push_str(", ");
                        self.advance();
                    } else if self.current_token() == &Token::RBrace {
                        break;
                    } else {
                        return Err("Expected ',' or '}' in braced import".to_string());
                    }
                } else if self.current_token() == &Token::RBrace {
                    break;
                } else {
                    return Err("Expected identifier in braced import".to_string());
                }
            }

            self.expect(Token::RBrace)?;
            path_str.push('}');
        }

        // Split the path string into segments
        // Examples:
        // - "std::fs" -> ["std", "fs"]
        // - "self::utils" -> ["self", "utils"]
        // - "./module::Type" -> ["./module", "Type"]
        // - "module::{A, B, C}" -> ["module::{A, B, C}"] (keep braced imports as one segment)

        if path_str.contains("::{") {
            // Braced import - keep as single segment
            path.push(path_str.clone());
        } else if path_str.starts_with("./") || path_str.starts_with("../") {
            // Relative import - split on :: but keep ./ or ../ prefix with first segment
            let parts: Vec<&str> = path_str.split("::").collect();
            for part in parts {
                path.push(part.to_string());
            }
        } else {
            // Absolute import - split on ::
            let parts: Vec<&str> = path_str.split("::").collect();
            for part in parts {
                if !part.is_empty() {
                    path.push(part.to_string());
                }
            }
        }

        // Check for optional "as alias" syntax
        let alias = if self.current_token() == &Token::As {
            self.advance();

            // Parse the alias identifier
            if let Token::Ident(alias_name) = self.current_token() {
                let alias = alias_name.clone();
                self.advance();
                Some(alias)
            } else {
                return Err("Expected alias identifier after 'as'".to_string());
            }
        } else {
            None
        };

        Ok((path, alias))
    }

    pub(crate) fn parse_mod(&mut self) -> Result<(String, Vec<Item<'static>>, bool), String> {
        // Note: Token::Mod already consumed in parse_item

        // Get module name
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected module name after 'mod'".to_string());
        };

        // Check for external module (mod name) vs inline module (mod name { ... })
        // Semicolons are optional thanks to ASI
        if self.current_token() == &Token::LBrace {
            // Inline module: mod name { ... }
            self.expect(Token::LBrace)?;

            let mut items = Vec::new();
            while self.current_token() != &Token::RBrace && self.current_token() != &Token::Eof {
                items.push(self.parse_item()?);

                // Consume optional semicolon after items (ASI - semicolons are optional)
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                }
            }

            self.expect(Token::RBrace)?;

            // is_public will be set by parse_item if pub keyword was present
            Ok((name, items, false))
        } else {
            // External module: mod name (semicolon optional)
            // Consume optional semicolon
            if self.current_token() == &Token::Semicolon {
                self.advance();
            }
            // Return empty items list - the module will be resolved by the module system
            Ok((name, Vec::new(), false))
        }
    }
}
