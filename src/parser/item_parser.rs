// Item Parser - Windjammer Top-Level Item Parsing Functions
//
// This module contains functions for parsing top-level items in Windjammer.
// Items include functions, structs, enums, traits, impl blocks, and their components.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_impl(&mut self) -> Result<ImplBlock, String> {
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
        let (trait_name, trait_type_args, type_name) = if self.current_token() == &Token::For {
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

            // Capture doc comment if present (/// or //!)
            let doc_comment = if let Token::DocComment(comment) = self.current_token() {
                let comment = comment.clone();
                self.advance();
                Some(comment)
            } else {
                None
            };

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
            func.is_pub = is_pub;
            func.is_async = is_async;
            func.decorators = decorators;
            func.doc_comment = doc_comment; // Doc comment from before the method
            func.parent_type = Some(type_name.clone()); // Track which impl block this function belongs to
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
        })
    }

    pub(crate) fn parse_trait(&mut self) -> Result<TraitDecl, String> {
        // Parse: trait Name<T, U> { methods }
        let name = if let Token::Ident(n) = self.current_token() {
            let n = n.clone();
            self.advance();
            n
        } else {
            return Err("Expected trait name".to_string());
        };

        // Parse optional generic parameters
        let generics = if self.current_token() == &Token::Lt {
            self.advance();
            let mut params = Vec::new();

            while self.current_token() != &Token::Gt {
                if let Token::Ident(param) = self.current_token() {
                    params.push(param.clone());
                    self.advance();

                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                } else {
                    return Err("Expected generic parameter name".to_string());
                }
            }

            self.expect_gt_or_split_shr()?; // Handle nested generics
            params
        } else {
            Vec::new()
        };

        // Parse optional supertraits: trait Manager: Employee + Person { ... }
        let supertraits = if self.current_token() == &Token::Colon {
            self.advance(); // consume ':'
            let mut traits = Vec::new();

            loop {
                if let Token::Ident(trait_name) = self.current_token() {
                    traits.push(trait_name.clone());
                    self.advance();

                    if self.current_token() == &Token::Plus {
                        self.advance(); // consume '+'
                    } else {
                        break;
                    }
                } else {
                    return Err("Expected supertrait name after ':'".to_string());
                }
            }

            traits
        } else {
            Vec::new()
        };

        self.expect(Token::LBrace)?;

        let mut associated_types = Vec::new();
        let mut methods = Vec::new();

        while self.current_token() != &Token::RBrace {
            // Check if this is an associated type declaration: type Name;
            if self.current_token() == &Token::Type {
                self.advance(); // consume 'type'

                let assoc_name = if let Token::Ident(n) = self.current_token() {
                    let name = n.clone();
                    self.advance();
                    name
                } else {
                    return Err("Expected associated type name".to_string());
                };

                // Semicolons are optional for associated types (like Swift, Kotlin, Go)
                if self.current_token() == &Token::Semicolon {
                    self.advance(); // consume optional semicolon
                }

                associated_types.push(AssociatedType {
                    name: assoc_name,
                    concrete_type: None, // No concrete type in trait declaration
                });

                continue;
            }

            // Capture doc comment if present (/// or //!)
            let doc_comment = if let Token::DocComment(comment) = self.current_token() {
                let comment = comment.clone();
                self.advance();
                Some(comment)
            } else {
                None
            };

            // Parse trait method signature
            let is_async = if self.current_token() == &Token::Async {
                self.advance();
                true
            } else {
                false
            };

            self.expect(Token::Fn)?;

            let method_name = if let Token::Ident(n) = self.current_token() {
                let n = n.clone();
                self.advance();
                n
            } else {
                return Err("Expected method name in trait".to_string());
            };

            self.expect(Token::LParen)?;
            let parameters = self.parse_parameters()?;
            self.expect(Token::RParen)?;

            let return_type = if self.current_token() == &Token::Arrow {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };

            // Check for default implementation (optional body)
            let body = if self.current_token() == &Token::LBrace {
                self.advance();
                let statements = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                Some(statements)
            } else {
                // No body - this is a trait method declaration
                // Semicolons are optional (Windjammer philosophy: minimize ceremony)
                if self.current_token() == &Token::Semicolon {
                    self.advance(); // consume optional semicolon
                }
                None
            };

            methods.push(TraitMethod {
                name: method_name,
                parameters,
                return_type,
                is_async,
                body,
                doc_comment,
            });
        }

        self.expect(Token::RBrace)?;

        Ok(TraitDecl {
            name,
            generics,
            supertraits,
            associated_types,
            methods,
            doc_comment: None,
        })
    }

    pub(crate) fn parse_decorator<'parser>(&'parser mut self) -> Result<Decorator<'parser>, String> {
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

    fn parse_decorator_arguments<'parser>(&'parser mut self) -> Result<Vec<(String, Expression<'parser>)>, String> {
        let mut args = Vec::new();

        while self.current_token() != &Token::RParen {
            // Check if it's a named argument (key: value)
            if let Token::Ident(key) = self.current_token() {
                let key = key.clone();
                self.advance();

                if self.current_token() == &Token::Colon {
                    self.advance();
                    let value = self.parse_expression()?;
                    args.push((key, value));
                } else {
                    // Positional argument (just a string or expression)
                    // Reparse as expression
                    let expr = Expression::Identifier {
                        name: key,
                        location: self.current_location(),
                    };
                    args.push((String::new(), expr));
                }
            } else {
                // Positional expression argument
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
            let name_opt = match self.current_token() {
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

    pub(crate) fn parse_mod(&mut self) -> Result<(String, Vec<Item>, bool), String> {
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

    pub(crate) fn parse_function(&mut self) -> Result<FunctionDecl, String> {
        // Note: Token::Fn already consumed in parse_item

        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected function name".to_string());
        };

        // Parse type parameters: fn foo<T, U>(...)
        let type_params = self.parse_type_params()?;

        self.expect(Token::LParen)?;
        let parameters = self.parse_parameters()?;
        self.expect(Token::RParen)?;

        let return_type = if self.current_token() == &Token::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse where clause (optional): where T: Display, U: Debug
        let where_clause = self.parse_where_clause()?;

        // Parse body (or semicolon for extern functions)
        // Semicolons are optional (Windjammer philosophy)
        let body = if self.current_token() == &Token::Semicolon {
            self.advance();
            Vec::new() // Empty body for extern functions
        } else if self.current_token() == &Token::LBrace {
            self.expect(Token::LBrace)?;
            let statements = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;
            statements
        } else {
            // No semicolon and no body - assume extern function
            Vec::new()
        };

        Ok(FunctionDecl {
            name,
            is_pub: false,          // Set by parse_item if pub keyword present
            is_extern: false,       // Set by parse_item if extern keyword present
            type_params,            // Parsed generic type parameters
            where_clause,           // Parsed where clause
            decorators: Vec::new(), // Set by parse_item
            is_async: false,        // Set by parse_item
            parameters,
            return_type,
            body,
            parent_type: None, // Set by parse_impl for methods
            doc_comment: None, // Set by parse_item if doc comments present
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, String> {
        let mut params = Vec::new();

        while self.current_token() != &Token::RParen {
            // Check for self parameters
            if self.current_token() == &Token::Ampersand {
                self.advance();
                if self.current_token() == &Token::Mut {
                    self.advance();
                    self.expect(Token::Self_)?;
                    params.push(Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Mut,
                        is_mutable: false,
                    });
                } else {
                    self.expect(Token::Self_)?;
                    params.push(Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Ref,
                        is_mutable: false,
                    });
                }
            } else if self.current_token() == &Token::Self_ {
                self.advance();
                params.push(Parameter {
                    name: "self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    ownership: OwnershipHint::Owned,
                    is_mutable: false,
                });
            } else if self.current_token() == &Token::Mut && self.peek(1) == Some(&Token::Self_) {
                // mut self (owned mutable) - only if next token is Self_
                self.advance(); // consume mut
                self.advance(); // consume self
                params.push(Parameter {
                    name: "self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    ownership: OwnershipHint::Owned,
                    is_mutable: true,
                });
            } else {
                // Regular parameter - could be a simple name or a pattern
                // Check if this is a pattern parameter (starts with '(')
                if self.current_token() == &Token::LParen {
                    // Parse tuple pattern
                    let pattern = self.parse_pattern()?;
                    self.expect(Token::Colon)?;
                    let type_ = self.parse_type()?;

                    // Extract a name from the pattern for backward compatibility
                    let name = Self::pattern_to_name(&pattern);

                    // CRITICAL FIX: Determine ownership from the type annotation
                    // If the type is explicitly &T or &mut T, use that.
                    // Otherwise, treat it as owned (pass by value).
                    let ownership = match &type_ {
                        Type::Reference(_) => OwnershipHint::Ref,
                        Type::MutableReference(_) => OwnershipHint::Mut,
                        _ => OwnershipHint::Inferred, // Let analyzer infer ownership based on usage
                    };

                    params.push(Parameter {
                        name,
                        pattern: Some(pattern),
                        type_,
                        ownership,
                        is_mutable: false,
                    });
                } else {
                    // Simple identifier parameter
                    // Check for 'mut' keyword and preserve it
                    let is_mutable = if self.current_token() == &Token::Mut {
                        self.advance();
                        true
                    } else {
                        false
                    };

                    let name = if let Token::Ident(n) = self.current_token() {
                        let name = n.clone();
                        self.advance();
                        name
                    } else {
                        return Err(format!(
                            "Expected parameter name (at token position {})",
                            self.position
                        ));
                    };

                    self.expect(Token::Colon)?;
                    let type_ = self.parse_type()?;

                    // CRITICAL FIX: Determine ownership from the type annotation
                    // If the type is explicitly &T or &mut T, use that.
                    // Otherwise, let the analyzer infer based on usage.
                    // This allows the analyzer to automatically add &mut when parameters are mutated.
                    let ownership = match &type_ {
                        Type::Reference(_) => OwnershipHint::Ref,
                        Type::MutableReference(_) => OwnershipHint::Mut,
                        _ => OwnershipHint::Inferred, // Let analyzer infer ownership based on usage
                    };

                    params.push(Parameter {
                        name,
                        pattern: None,
                        type_,
                        ownership,
                        is_mutable,
                    });
                }
            }

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    // ------------------------------------------------------------------------
    // TYPE PARSING (used by multiple sections above)
    // ------------------------------------------------------------------------

    pub(crate) fn parse_struct(&mut self) -> Result<StructDecl, String> {
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
            type_params,
            where_clause,
            fields,
            decorators: Vec::new(),
            doc_comment: None, // Set by parse_item if doc comments present
        })
    }

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

            let variant_name = if let Token::Ident(n) = self.current_token() {
                let name = n.clone();
                self.advance();
                name
            } else {
                return Err("Expected variant name".to_string());
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
