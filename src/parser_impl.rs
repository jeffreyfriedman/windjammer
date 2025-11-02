// Parser - Windjammer Language Parser
//
// This file contains the complete parser for Windjammer. It is organized into the following sections:
//
// 1. AST TYPES (lines ~3-340)
//    - Type, TypeParam, Parameter, FunctionDecl, StructDecl, EnumDecl, TraitDecl, ImplBlock
//    - Expression, Statement, Pattern, Item, Program
//
// 2. PARSER CORE (lines ~344-400)
//    - Parser struct
//    - Basic utilities: new(), current_token(), advance(), expect(), peek()
//    - Helper: type_to_string()
//
// 3. TOP-LEVEL PARSING (lines ~400-700)
//    - parse() - main entry point
//    - parse_item() - dispatches to item parsers
//    - parse_const_or_static()
//    - parse_use()
//    - parse_decorator() and parse_decorator_arguments()
//
// 4. ITEM PARSING (lines ~700-1500)
//    - parse_impl() - impl blocks with generics and trait impls
//    - parse_trait() - trait definitions
//    - parse_function() - function declarations
//    - parse_parameters()
//    - parse_struct() - struct definitions
//    - parse_enum() - enum definitions with generics
//    - parse_type_params() - generic type parameters with bounds
//    - parse_where_clause() - where clauses
//
// 5. STATEMENT PARSING (lines ~1500-1900)
//    - parse_block_statements()
//    - parse_statement() - dispatches to statement parsers
//    - parse_const_statement(), parse_static_statement()
//    - parse_let(), parse_return()
//    - parse_if(), parse_match()
//    - parse_for(), parse_loop(), parse_while()
//    - parse_go(), parse_defer()
//
// 6. PATTERN PARSING (lines ~1900-2000)
//    - parse_pattern_with_or() - OR patterns
//    - parse_pattern() - all pattern types including enum variants
//
// 7. EXPRESSION PARSING (lines ~2000-2800)
//    - parse_expression() - entry point
//    - parse_ternary_expression() - ternary operator
//    - parse_match_value() - match value with special handling
//    - parse_binary_expression() - operator precedence climbing
//    - get_binary_op() - operator precedence table
//    - parse_primary_expression() - literals, identifiers, calls, etc.
//    - parse_postfix_expression() - method calls, field access, indexing, turbofish
//    - parse_arguments()
//    - parse_closure()
//
// 8. TYPE PARSING (lines ~2800+)
//    - parse_type() - all type variants
//
// TODO: Split this into modules:
//   - parser/mod.rs - Parser struct and utilities
//   - parser/types.rs - Type parsing
//   - parser/patterns.rs - Pattern parsing
//   - parser/expressions.rs - Expression parsing
//   - parser/statements.rs - Statement parsing
//   - parser/items.rs - Top-level item parsing

use crate::lexer::Token;

// Import all AST types from the new parser::ast module
pub use crate::parser::ast::*;

// ============================================================================
// SECTION 2: PARSER CORE
// ============================================================================

pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) position: usize,
    #[allow(dead_code)]
    pub(crate) filename: String,
    #[allow(dead_code)]
    pub(crate) source: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
            filename: String::new(),
            source: String::new(),
        }
    }

    pub fn new_with_source(tokens: Vec<Token>, filename: String, source: String) -> Self {
        Parser {
            tokens,
            position: 0,
            filename,
            source,
        }
    }

    pub(crate) fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    pub(crate) fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    pub(crate) fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} (at token position {})",
                expected,
                self.current_token(),
                self.position
            ))
        }
    }

    // ========================================================================
    // SECTION 3: TOP-LEVEL PARSING
    // ========================================================================

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();

        while self.current_token() != &Token::Eof {
            items.push(self.parse_item()?);
        }

        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        // Check for decorators
        let mut decorators = Vec::new();
        while let Token::Decorator(_) = self.current_token() {
            decorators.push(self.parse_decorator()?);
        }

        // Check for pub keyword (for module functions)
        let is_pub = if self.current_token() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        };

        match self.current_token() {
            Token::Fn => {
                self.advance(); // Consume the Fn token
                let mut func = self.parse_function()?;
                func.decorators = decorators.clone();
                // Check if @async decorator is present
                if decorators.iter().any(|d| d.name == "async") {
                    func.is_async = true;
                }
                Ok(Item::Function(func))
            }
            Token::Async => {
                self.advance();
                self.expect(Token::Fn)?;
                let mut func = self.parse_function()?;
                func.is_async = true;
                func.decorators = decorators;
                Ok(Item::Function(func))
            }
            Token::Struct => {
                self.advance();
                let mut struct_decl = self.parse_struct()?;
                struct_decl.decorators = decorators;
                Ok(Item::Struct(struct_decl))
            }
            Token::Enum => {
                self.advance();
                Ok(Item::Enum(self.parse_enum()?))
            }
            Token::Trait => {
                self.advance();
                Ok(Item::Trait(self.parse_trait()?))
            }
            Token::Impl => {
                self.advance();
                let mut impl_block = self.parse_impl()?;
                impl_block.decorators = decorators;
                Ok(Item::Impl(impl_block))
            }
            Token::Const => {
                self.advance();
                let (name, type_, value) = self.parse_const_or_static()?;
                // For now, we don't store is_pub in the AST (future enhancement)
                // But at least we parse it correctly
                let _ = is_pub; // Suppress unused warning
                Ok(Item::Const { name, type_, value })
            }
            Token::Static => {
                self.advance();
                let mutable = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let (name, type_, value) = self.parse_const_or_static()?;
                Ok(Item::Static {
                    name,
                    mutable,
                    type_,
                    value,
                })
            }
            Token::Use => {
                self.advance(); // consume 'use'
                let (path, alias) = self.parse_use()?;
                Ok(Item::Use { path, alias })
            }
            Token::Bound => {
                self.advance(); // consume 'bound'
                self.parse_bound_alias()
            }
            _ => Err(format!(
                "Unexpected token: {:?} (at token position {})",
                self.current_token(),
                self.position
            )),
        }
    }

    fn parse_bound_alias(&mut self) -> Result<Item, String> {
        // bound Name = Trait + Trait + ...
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected bound alias name".to_string());
        };

        self.expect(Token::Assign)?;

        // Parse trait list: Trait + Trait + ...
        let mut traits = Vec::new();
        loop {
            if let Token::Ident(trait_name) = self.current_token() {
                traits.push(trait_name.clone());
                self.advance();
            } else {
                return Err("Expected trait name in bound alias".to_string());
            }

            if self.current_token() == &Token::Plus {
                self.advance(); // consume +
            } else {
                break;
            }
        }

        Ok(Item::BoundAlias { name, traits })
    }

    fn parse_const_or_static(&mut self) -> Result<(String, Type, Expression), String> {
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected const/static name".to_string());
        };

        self.expect(Token::Colon)?;
        let type_ = self.parse_type()?;

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;

        Ok((name, type_, value))
    }

    // ========================================================================
    // SECTION 4: ITEM PARSING (Functions, Structs, Enums, Traits, Impls)
    // ========================================================================

    fn parse_impl(&mut self) -> Result<ImplBlock, String> {
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

                self.expect(Token::Semicolon)?;

                associated_types.push(AssociatedType {
                    name: assoc_name,
                    concrete_type: Some(concrete_type),
                });

                continue;
            }

            // Skip decorators for now (could be added later)
            let mut decorators = Vec::new();
            while let Token::Decorator(_) = self.current_token() {
                decorators.push(self.parse_decorator()?);
            }

            // Parse function (pub optional)
            if self.current_token() == &Token::Pub {
                self.advance();
            }

            let is_async = if self.current_token() == &Token::Async {
                self.advance();
                true
            } else {
                false
            };

            self.expect(Token::Fn)?;
            let mut func = self.parse_function()?;
            func.is_async = is_async;
            func.decorators = decorators;
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

    fn parse_trait(&mut self) -> Result<TraitDecl, String> {
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

            self.expect(Token::Gt)?;
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

                self.expect(Token::Semicolon)?;

                associated_types.push(AssociatedType {
                    name: assoc_name,
                    concrete_type: None, // No concrete type in trait declaration
                });

                continue;
            }

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
                None
            };

            methods.push(TraitMethod {
                name: method_name,
                parameters,
                return_type,
                is_async,
                body,
            });
        }

        self.expect(Token::RBrace)?;

        Ok(TraitDecl {
            name,
            generics,
            supertraits,
            associated_types,
            methods,
        })
    }

    fn parse_decorator(&mut self) -> Result<Decorator, String> {
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

    fn parse_decorator_arguments(&mut self) -> Result<Vec<(String, Expression)>, String> {
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
                    let expr = Expression::Identifier(key);
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

    fn parse_use(&mut self) -> Result<(Vec<String>, Option<String>), String> {
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
            // Allow keywords as identifiers in module paths
            let name_opt = match self.current_token() {
                Token::Ident(n) => Some(n.clone()),
                Token::Thread => Some("thread".to_string()),
                Token::Async => Some("async".to_string()),
                _ => None,
            };

            if let Some(name) = name_opt {
                path_str.push_str(&name);
                self.advance();

                // Check for :: or / as separator (. is NOT supported - use :: for modules)
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
                    path_str.push('/');
                    self.advance();
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

        // For now, return the path as a single-element vector
        // This preserves the relative path structure
        path.push(path_str);

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

    fn parse_function(&mut self) -> Result<FunctionDecl, String> {
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

        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(FunctionDecl {
            name,
            type_params,            // Parsed generic type parameters
            where_clause,           // Parsed where clause
            decorators: Vec::new(), // Set by parse_item
            is_async: false,        // Set by parse_item
            parameters,
            return_type,
            body,
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
                    });
                } else {
                    self.expect(Token::Self_)?;
                    params.push(Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Ref,
                    });
                }
            } else if self.current_token() == &Token::Self_ {
                self.advance();
                params.push(Parameter {
                    name: "self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    ownership: OwnershipHint::Owned,
                });
            } else if self.current_token() == &Token::Mut && self.peek(1) == Some(&Token::Self_) {
                // mut self (owned mutable) - only if next token is Self_
                self.advance(); // consume mut
                self.advance(); // consume self
                params.push(Parameter {
                    name: "mut self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    ownership: OwnershipHint::Owned,
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

                    // Owned parameters are the default in Windjammer
                    // (References are already explicit in the type)
                    let ownership = OwnershipHint::Owned;

                    params.push(Parameter {
                        name,
                        pattern: Some(pattern),
                        type_,
                        ownership,
                    });
                } else {
                    // Simple identifier parameter
                    // Optional: consume 'mut' keyword (for backward compatibility)
                    // In Windjammer, owned parameters are auto-mutable, so 'mut' is redundant
                    if self.current_token() == &Token::Mut {
                        self.advance();
                    }

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

                    // Owned parameters are the default in Windjammer
                    // (References are already explicit in the type)
                    let ownership = OwnershipHint::Owned;

                    params.push(Parameter {
                        name,
                        pattern: None,
                        type_,
                        ownership,
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

    fn parse_struct(&mut self) -> Result<StructDecl, String> {
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

        self.expect(Token::LBrace)?;

        let mut fields = Vec::new();
        while self.current_token() != &Token::RBrace {
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
            });

            if self.current_token() == &Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;

        Ok(StructDecl {
            name,
            type_params,
            where_clause,
            fields,
            decorators: Vec::new(),
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDecl, String> {
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
            let variant_name = if let Token::Ident(n) = self.current_token() {
                let name = n.clone();
                self.advance();
                name
            } else {
                return Err("Expected variant name".to_string());
            };

            let data = if self.current_token() == &Token::LParen {
                // Tuple-style variant: Variant(Type)
                self.advance();
                let type_ = self.parse_type()?;
                self.expect(Token::RParen)?;
                Some(type_)
            } else if self.current_token() == &Token::LBrace {
                // Struct-style variant: Variant { field1: Type1, field2: Type2 }
                // For now, we'll parse this as a tuple containing a struct type
                // TODO: Extend EnumVariant to properly represent struct-style variants
                self.advance(); // consume {

                // Skip the struct fields for now - just consume until we hit }
                let mut depth = 1;
                while depth > 0 && self.current_token() != &Token::Eof {
                    match self.current_token() {
                        Token::LBrace => depth += 1,
                        Token::RBrace => depth -= 1,
                        _ => {}
                    }
                    if depth > 0 {
                        self.advance();
                    }
                }
                self.expect(Token::RBrace)?;

                // Represent as None for now - struct-style variants don't have a single type
                None
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                data,
            });

            if self.current_token() == &Token::Comma {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;

        Ok(EnumDecl {
            name,
            type_params,
            variants,
        })
    }

    // ========================================================================
    // SECTION 5: STATEMENT PARSING (Let, If, Match, For, While, etc.)
    // ========================================================================

    pub(crate) fn parse_block_statements(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();

        while self.current_token() != &Token::RBrace && self.current_token() != &Token::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    pub(crate) fn parse_statement(&mut self) -> Result<Statement, String> {
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
                    Ok(Statement::Expression(expr))
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
                    Ok(Statement::Expression(expr))
                }
            }
            Token::Defer => self.parse_defer(),
            Token::Break => {
                self.advance();
                Ok(Statement::Break)
            }
            Token::Continue => {
                self.advance();
                Ok(Statement::Continue)
            }
            Token::Use => {
                self.advance(); // consume 'use'
                let (path, alias) = self.parse_use()?;
                Ok(Statement::Use { path, alias })
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

                        Ok(Statement::Assignment {
                            target: expr,
                            value,
                        })
                    }
                    Token::PlusAssign
                    | Token::MinusAssign
                    | Token::StarAssign
                    | Token::SlashAssign
                    | Token::PercentAssign => {
                        let op_token = self.current_token().clone();
                        self.advance(); // consume compound operator

                        let rhs = self.parse_expression()?;

                        // Convert x += y to x = x + y
                        let op = match op_token {
                            Token::PlusAssign => BinaryOp::Add,
                            Token::MinusAssign => BinaryOp::Sub,
                            Token::StarAssign => BinaryOp::Mul,
                            Token::SlashAssign => BinaryOp::Div,
                            Token::PercentAssign => BinaryOp::Mod,
                            _ => unreachable!(),
                        };

                        let value = Expression::Binary {
                            left: Box::new(expr.clone()),
                            op,
                            right: Box::new(rhs),
                        };

                        // Optionally consume semicolon
                        if self.current_token() == &Token::Semicolon {
                            self.advance();
                        }

                        Ok(Statement::Assignment {
                            target: expr,
                            value,
                        })
                    }
                    _ => {
                        // Optionally consume semicolon after expression statement
                        if self.current_token() == &Token::Semicolon {
                            self.advance();
                        }
                        Ok(Statement::Expression(expr))
                    }
                }
            }
        }
    }

    fn parse_const_statement(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'const'
        let (name, type_, value) = self.parse_const_or_static()?;
        Ok(Statement::Const { name, type_, value })
    }

    fn parse_static_statement(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'static'
        let mutable = if self.current_token() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        let (name, type_, value) = self.parse_const_or_static()?;
        Ok(Statement::Static {
            name,
            mutable,
            type_,
            value,
        })
    }

    fn parse_for(&mut self) -> Result<Statement, String> {
        self.expect(Token::For)?;

        // Parse pattern: identifier, reference pattern (&x), or tuple pattern like (idx, item)
        let pattern = if self.current_token() == &Token::Ampersand {
            // Reference pattern: &x
            self.advance(); // consume &
            if let Token::Ident(name) = self.current_token() {
                let name = name.clone();
                self.advance();
                Pattern::Reference(Box::new(Pattern::Identifier(name)))
            } else {
                return Err("Expected identifier after & in for loop pattern".to_string());
            }
        } else if self.current_token() == &Token::LParen {
            // Tuple pattern
            self.advance(); // consume (
            let mut patterns = Vec::new();

            while self.current_token() != &Token::RParen {
                // Support reference patterns in tuples too: (&x, &y)
                let pat = if self.current_token() == &Token::Ampersand {
                    self.advance();
                    if let Token::Ident(name) = self.current_token() {
                        let name = name.clone();
                        self.advance();
                        Pattern::Reference(Box::new(Pattern::Identifier(name)))
                    } else {
                        return Err("Expected identifier after & in tuple pattern".to_string());
                    }
                } else if let Token::Ident(name) = self.current_token() {
                    let name = name.clone();
                    self.advance();
                    Pattern::Identifier(name)
                } else {
                    return Err("Expected identifier in tuple pattern".to_string());
                };

                patterns.push(pat);

                if self.current_token() == &Token::Comma {
                    self.advance();
                }
            }

            self.expect(Token::RParen)?;
            Pattern::Tuple(patterns)
        } else if let Token::Ident(name) = self.current_token() {
            let name = name.clone();
            self.advance();
            Pattern::Identifier(name)
        } else {
            return Err(
                "Expected variable name, reference pattern, or tuple pattern in for loop"
                    .to_string(),
            );
        };

        self.expect(Token::In)?;
        let iterable = self.parse_expression()?;

        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::For {
            pattern,
            iterable,
            body,
        })
    }

    fn parse_thread(&mut self) -> Result<Statement, String> {
        self.expect(Token::Thread)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::Thread { body })
    }

    fn parse_async(&mut self) -> Result<Statement, String> {
        self.expect(Token::Async)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::Async { body })
    }

    fn parse_defer(&mut self) -> Result<Statement, String> {
        self.expect(Token::Defer)?;
        let stmt = self.parse_statement()?;

        Ok(Statement::Defer(Box::new(stmt)))
    }

    fn parse_let(&mut self) -> Result<Statement, String> {
        self.expect(Token::Let)?;

        let mutable = if self.current_token() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };

        // Parse pattern (could be simple name, wildcard, or tuple destructuring)
        let pattern = if self.current_token() == &Token::LParen {
            // Tuple destructuring: let (x, y) = ...
            self.parse_pattern()?
        } else if self.current_token() == &Token::Underscore {
            // Wildcard: let _ = ...
            self.advance();
            Pattern::Wildcard
        } else if let Token::Ident(n) = self.current_token() {
            // Simple variable: let x = ...
            let name = n.clone();
            self.advance();
            Pattern::Identifier(name)
        } else {
            return Err(format!(
                "Expected variable name or pattern (at token position {})",
                self.position
            ));
        };

        let type_ = if self.current_token() == &Token::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;

        // Optionally consume semicolon (semicolons are optional in Windjammer)
        if self.current_token() == &Token::Semicolon {
            self.advance();
        }

        Ok(Statement::Let {
            pattern,
            mutable,
            type_,
            value,
        })
    }

    fn parse_return(&mut self) -> Result<Statement, String> {
        self.advance();

        if matches!(self.current_token(), Token::RBrace | Token::Semicolon) {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expression()?)))
        }
    }

    pub(crate) fn parse_if(&mut self) -> Result<Statement, String> {
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
            let mut arms = vec![MatchArm {
                pattern,
                guard: None,
                body: Expression::Block(then_block),
            }];

            // Add wildcard arm for else block
            if let Some(else_stmts) = else_block {
                arms.push(MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Expression::Block(else_stmts),
                });
            }

            Ok(Statement::Match { value, arms })
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

            Ok(Statement::If {
                condition,
                then_block,
                else_block,
            })
        }
    }

    fn parse_match(&mut self) -> Result<Statement, String> {
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

        Ok(Statement::Match { value, arms })
    }

    // ========================================================================
    // SECTION 6: PATTERN PARSING
    // ========================================================================

    fn parse_loop(&mut self) -> Result<Statement, String> {
        self.expect(Token::Loop)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;

        Ok(Statement::Loop { body })
    }

    fn parse_while(&mut self) -> Result<Statement, String> {
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
            let match_stmt = Statement::Match {
                value: expr.clone(),
                arms: vec![
                    MatchArm {
                        pattern,
                        guard: None,
                        body: Expression::Block(body.clone()),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Expression::Block(vec![Statement::Break]),
                    },
                ],
            };

            Ok(Statement::Loop {
                body: vec![match_stmt],
            })
        } else {
            // Regular while loop
            let condition = self.parse_expression()?;

            self.expect(Token::LBrace)?;
            let body = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;

            Ok(Statement::While { condition, body })
        }
    }

    // Helper: Extract a name from a pattern for backward compatibility

    // Public wrapper methods for component compiler
    pub fn parse_expression_public(&mut self) -> Result<Expression, String> {
        self.parse_expression()
    }

    pub fn parse_function_public(&mut self) -> Result<FunctionDecl, String> {
        self.parse_function()
    }
}
