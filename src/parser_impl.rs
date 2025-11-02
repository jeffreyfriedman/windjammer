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
    tokens: Vec<Token>,
    position: usize,
    #[allow(dead_code)]
    filename: String,
    #[allow(dead_code)]
    source: String,
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

    fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    // Helper to convert Type AST back to string for impl parsing
    #[allow(clippy::only_used_in_recursion)]
    fn type_to_string(&self, ty: &Type) -> String {
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

    fn expect(&mut self, expected: Token) -> Result<(), String> {
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

    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        // Parse generic type parameters: <T>, <T: Display>, <T: Display + Clone, U: Debug>
        if self.current_token() != &Token::Lt {
            return Ok(Vec::new());
        }

        self.advance(); // consume <
        let mut params = Vec::new();

        loop {
            if let Token::Ident(name) = self.current_token() {
                let param_name = name.clone();
                self.advance();

                // Check for trait bounds: T: Display
                let mut bounds = Vec::new();
                if self.current_token() == &Token::Colon {
                    self.advance(); // consume :

                    // Parse trait bounds separated by +
                    loop {
                        if let Token::Ident(trait_name) = self.current_token() {
                            bounds.push(trait_name.clone());
                            self.advance();

                            // Check for + (multiple bounds)
                            if self.current_token() == &Token::Plus {
                                self.advance();
                                continue;
                            } else {
                                break;
                            }
                        } else {
                            return Err("Expected trait name in bound".to_string());
                        }
                    }
                }

                params.push(TypeParam {
                    name: param_name,
                    bounds,
                });

                if self.current_token() == &Token::Comma {
                    self.advance();
                } else if self.current_token() == &Token::Gt {
                    self.advance();
                    break;
                } else {
                    return Err("Expected ',' or '>' in type parameters".to_string());
                }
            } else {
                return Err("Expected type parameter name".to_string());
            }
        }

        Ok(params)
    }

    fn parse_where_clause(&mut self) -> Result<Vec<(String, Vec<String>)>, String> {
        // Parse where clause: where T: Display, U: Debug + Clone
        if self.current_token() != &Token::Where {
            return Ok(Vec::new());
        }

        self.advance(); // consume 'where'
        let mut clauses = Vec::new();

        loop {
            // Parse type parameter name
            if let Token::Ident(type_param) = self.current_token() {
                let param_name = type_param.clone();
                self.advance();

                // Expect colon
                if self.current_token() != &Token::Colon {
                    return Err("Expected ':' after type parameter in where clause".to_string());
                }
                self.advance();

                // Parse trait bounds separated by +
                let mut bounds = Vec::new();
                loop {
                    if let Token::Ident(trait_name) = self.current_token() {
                        bounds.push(trait_name.clone());
                        self.advance();

                        // Check for + (multiple bounds)
                        if self.current_token() == &Token::Plus {
                            self.advance();
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        return Err("Expected trait name in where clause".to_string());
                    }
                }

                clauses.push((param_name, bounds));

                // Check for comma (more clauses) or end
                if self.current_token() == &Token::Comma {
                    self.advance();
                    continue;
                } else {
                    // End of where clause
                    break;
                }
            } else {
                return Err("Expected type parameter name in where clause".to_string());
            }
        }

        Ok(clauses)
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

    fn parse_type(&mut self) -> Result<Type, String> {
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
                            if let Token::Ident(next_segment) = &self.tokens[self.position + 1] {
                                let next_segment_str = next_segment.clone(); // Clone before any mutable borrows

                                // Could be either:
                                // 1. Path segment: std::fs::File
                                // 2. Associated type: Self::Item

                                // For now, check if the next token after the identifier is a generic or end
                                // to determine if this is the final segment (associated type)
                                if self.position + 2 < self.tokens.len() {
                                    let after_next = &self.tokens[self.position + 2];
                                    match after_next {
                                        Token::Lt
                                        | Token::Comma
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
                if self.current_token() == &Token::Lt {
                    self.advance();

                    // Handle Vec<T>, Option<T>, Result<T, E>
                    if type_name == "Vec" {
                        let inner = Box::new(self.parse_type()?);
                        self.expect(Token::Gt)?;
                        Type::Vec(inner)
                    } else if type_name == "Option" {
                        let inner = Box::new(self.parse_type()?);
                        self.expect(Token::Gt)?;
                        Type::Option(inner)
                    } else if type_name == "Result" {
                        let ok_type = Box::new(self.parse_type()?);
                        self.expect(Token::Comma)?;
                        let err_type = Box::new(self.parse_type()?);
                        self.expect(Token::Gt)?;
                        Type::Result(ok_type, err_type)
                    } else {
                        // Generic custom type: Box<T>, HashMap<K, V>, etc.
                        let mut type_args = Vec::new();

                        loop {
                            type_args.push(self.parse_type()?);

                            if self.current_token() == &Token::Comma {
                                self.advance();
                            } else if self.current_token() == &Token::Gt {
                                self.advance();
                                break;
                            } else {
                                return Err("Expected ',' or '>' in type arguments".to_string());
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
            _ => return Err(format!("Expected type, got {:?}", self.current_token())),
        };

        Ok(base_type)
    }

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

    fn parse_block_statements(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();

        while self.current_token() != &Token::RBrace && self.current_token() != &Token::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
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

    fn parse_if(&mut self) -> Result<Statement, String> {
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

    fn parse_pattern_with_or(&mut self) -> Result<Pattern, String> {
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

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
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
                let name = name.clone();
                self.advance();

                // Check if it's a qualified enum variant: Result.Ok(x) or ClientMessage::Ping
                if self.current_token() == &Token::Dot || self.current_token() == &Token::ColonColon
                {
                    let separator = if self.current_token() == &Token::Dot {
                        "."
                    } else {
                        "::"
                    };
                    self.advance();

                    // Get variant name - must be an identifier
                    let variant = if let Token::Ident(v) = self.current_token() {
                        v.clone()
                    } else {
                        return Err(format!(
                            "Expected variant name after {}, got {:?}",
                            separator,
                            self.current_token()
                        ));
                    };
                    self.advance();

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
                        format!("{}{}{}", name, separator, variant),
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
                    Ok(Pattern::EnumVariant(name, binding))
                } else {
                    // Check if this could be an enum variant without parameters (None, Empty, etc.)
                    // For now, treat as identifier - the analyzer will determine if it's an enum variant
                    Ok(Pattern::Identifier(name))
                }
            }
            _ => Err(format!("Expected pattern, got {:?}", self.current_token())),
        }
    }

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

    // ========================================================================
    // SECTION 7: EXPRESSION PARSING
    // ========================================================================

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_ternary_expression()
    }

    fn parse_ternary_expression(&mut self) -> Result<Expression, String> {
        // Ternary operator removed - use if/else expressions instead
        // This simplifies the parser and eliminates ambiguity with TryOp (?)
        self.parse_binary_expression(0)
    }

    fn parse_match_value(&mut self) -> Result<Expression, String> {
        // Parse a non-struct-literal expression for match values
        // This is basically parse_binary_expression but without struct literal support
        let mut left = match self.current_token() {
            Token::LParen => {
                self.advance();

                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    return Ok(Expression::Tuple(vec![]));
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
                    Expression::Tuple(elements)
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
                    return Ok(Expression::Array(vec![]));
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
                Expression::Array(elements)
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
                }
            }
            Token::Star => {
                // Handle * dereference operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(inner),
                }
            }
            Token::Minus => {
                // Handle - negation operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(inner),
                }
            }
            Token::Bang => {
                // Handle ! not operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(inner),
                }
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();

                // Handle qualified paths with :: (e.g., std::fs::read)
                while self.current_token() == &Token::ColonColon {
                    // Look ahead to see if there's an identifier after ::
                    if self.position + 1 < self.tokens.len() {
                        if let Token::Ident(next_name) = &self.tokens[self.position + 1] {
                            // This is a qualified path segment
                            qualified_name.push_str("::");
                            qualified_name.push_str(next_name);
                            self.advance(); // consume ::
                            self.advance(); // consume identifier
                        } else if let Token::Lt = &self.tokens[self.position + 1] {
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
                Expression::Identifier(qualified_name)
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
                        left = Expression::Await(Box::new(left));
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
                            })
                        });

                        left = Expression::MethodCall {
                            object: Box::new(left),
                            method: "slice".to_string(),
                            type_args: None,
                            arguments: vec![
                                (None, Expression::Literal(Literal::Int(0))),
                                (None, *end_expr),
                            ],
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
                                })
                            });

                            left = Expression::MethodCall {
                                object: Box::new(left),
                                method: "slice".to_string(),
                                type_args: None,
                                arguments: vec![(None, *start_or_index), (None, *end_expr)],
                            };
                        } else {
                            // Regular index: [i]
                            self.expect(Token::RBracket)?;
                            left = Expression::Index {
                                object: Box::new(left),
                                index: start_or_index,
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
                            };
                        } else {
                            // Just a path, treat as field access
                            left = Expression::FieldAccess {
                                object: Box::new(left),
                                field: method,
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
                    Expression::Block(vec![Statement::Thread { body }])
                } else {
                    // Module path like thread::sleep_seconds
                    // Parse as identifier and let postfix operators handle ::
                    let name = "thread".to_string();
                    self.advance();
                    Expression::Identifier(name)
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
                    Expression::Block(vec![Statement::Async { body }])
                } else {
                    // Module path like async::something
                    let name = "async".to_string();
                    self.advance();
                    Expression::Identifier(name)
                }
            }
            Token::LeftArrow => {
                // Channel receive: <-ch
                self.advance();
                let channel = self.parse_primary_expression()?;
                Expression::ChannelRecv(Box::new(channel))
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
                }
            }
            Token::Star => {
                // Dereference: *expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(operand),
                }
            }
            Token::Minus => {
                // Negation: -expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                }
            }
            Token::Bang => {
                // Logical not: !expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                }
            }
            Token::Self_ => {
                // self keyword used in expressions
                self.advance();
                Expression::Identifier("self".to_string())
            }
            Token::IntLiteral(n) => {
                let n = *n;
                self.advance();
                Expression::Literal(Literal::Int(n))
            }
            Token::FloatLiteral(f) => {
                let f = *f;
                self.advance();
                Expression::Literal(Literal::Float(f))
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Expression::Literal(Literal::String(s))
            }
            Token::CharLiteral(c) => {
                let c = *c;
                self.advance();
                Expression::Literal(Literal::Char(c))
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
                                let tok = expr_lexer.next_token();
                                if tok == crate::lexer::Token::Eof {
                                    break;
                                }
                                expr_tokens.push(tok);
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
                let mut macro_args = vec![Expression::Literal(Literal::String(format_string))];
                macro_args.extend(args);

                Expression::MacroInvocation {
                    name: "format".to_string(),
                    args: macro_args,
                    delimiter: MacroDelimiter::Parens,
                }
            }
            Token::BoolLiteral(b) => {
                let b = *b;
                self.advance();
                Expression::Literal(Literal::Bool(b))
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();

                // Handle qualified paths with :: (e.g., sqlx::SqlitePool, std::fs::File)
                while self.current_token() == &Token::ColonColon {
                    // Look ahead to see if there's an identifier after ::
                    if self.position + 1 < self.tokens.len() {
                        if let Token::Ident(next_name) = &self.tokens[self.position + 1] {
                            // This is a qualified path segment
                            qualified_name.push_str("::");
                            qualified_name.push_str(next_name);
                            self.advance(); // consume ::
                            self.advance(); // consume identifier
                        } else if let Token::Lt = &self.tokens[self.position + 1] {
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
                            match &self.tokens[self.position + 1] {
                                Token::Ident(_) | Token::RBrace => {
                                    // Could be struct literal: { field: ... } or { field, ... } or { }
                                    if self.position + 2 < self.tokens.len() {
                                        matches!(
                                            &self.tokens[self.position + 2],
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
                            Expression::Identifier(field_name.clone())
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
                    }
                } else {
                    Expression::Identifier(qualified_name)
                }
            }
            Token::LParen => {
                self.advance();

                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    Expression::Tuple(vec![])
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
                        Expression::Tuple(exprs)
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
                    Expression::Array(vec![])
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
                        Expression::Array(elements)
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
                };
                Expression::Block(vec![match_stmt])
            }
            Token::Pipe => {
                // Closure: |params| body
                self.advance();
                let mut parameters = Vec::new();

                while self.current_token() != &Token::Pipe {
                    // Handle patterns like &x, &mut x, or just x
                    let param_name = match self.current_token() {
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
                    Box::new(Expression::Block(statements))
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
                        Box::new(Expression::Block(vec![stmt]))
                    } else {
                        // Reset and parse as a normal expression
                        self.position = checkpoint;
                        Box::new(self.parse_expression()?)
                    }
                };

                Expression::Closure { parameters, body }
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
                    Box::new(Expression::Block(statements))
                } else {
                    // Expression closure: || expr
                    Box::new(self.parse_expression()?)
                };

                Expression::Closure {
                    parameters: Vec::new(), // No parameters
                    body,
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
                        body: Expression::Block(then_block),
                    }];

                    if let Some(else_block) = else_block {
                        arms.push(MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Expression::Block(else_block),
                        });
                    }

                    let match_stmt = Statement::Match { value: expr, arms };

                    Expression::Block(vec![match_stmt])
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
                    };

                    Expression::Block(vec![if_stmt])
                }
            }
            Token::Unsafe => {
                // Unsafe block: unsafe { ... }
                self.advance(); // consume 'unsafe'
                self.expect(Token::LBrace)?;
                let body = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                Expression::Block(body)
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
                    // Empty map literal
                    return Ok(Expression::MapLiteral(vec![]));
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
                    let mut entries = vec![];

                    loop {
                        if self.current_token() == &Token::RBrace {
                            break;
                        }

                        let key = self.parse_ternary_expression()?;
                        self.expect(Token::Colon)?;
                        let value = self.parse_expression()?;

                        entries.push((key, value));

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
                    Expression::MapLiteral(entries)
                } else {
                    // Parse as block expression
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    Expression::Block(body)
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
                Expression::Block(vec![Statement::Return(return_value.map(|b| *b))])
            }
            // Allow certain keywords as identifiers in expression context (e.g., HTML attributes)
            Token::For => {
                self.advance();
                Expression::Identifier("for".to_string())
            }
            Token::Type => {
                self.advance();
                Expression::Identifier("type".to_string())
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
                        Expression::Await(Box::new(expr))
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
                                // Method call (possibly with turbofish)
                                self.advance();
                                let arguments = self.parse_arguments()?;
                                self.expect(Token::RParen)?;
                                Expression::MethodCall {
                                    object: Box::new(expr),
                                    method: field,
                                    type_args,
                                    arguments,
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
                            if let Expression::Identifier(name) = expr {
                                expr = Expression::Identifier(format!("{}{}", name, type_str));
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
                            }
                        } else {
                            return Err(format!(
                                "Expected '(', '::', or identifier after '::<Type>', got {:?}",
                                self.current_token()
                            ));
                        }
                    } else if let Token::Ident(method) = self.current_token() {
                        // Type::method or module::function (no turbofish)
                        let method = method.clone();
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
                            }
                        } else {
                            // Just a path, treat as field access
                            Expression::FieldAccess {
                                object: Box::new(expr),
                                field: method,
                            }
                        }
                    } else {
                        return Err(format!(
                            "Expected '<' or identifier after '::', got {:?}",
                            self.current_token()
                        ));
                    }
                }
                Token::LParen => {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(Token::RParen)?;
                    Expression::Call {
                        function: Box::new(expr),
                        arguments,
                    }
                }
                Token::Question => {
                    // TryOp: expr?
                    // No ambiguity since we removed ternary operator
                    self.advance();
                    Expression::TryOp(Box::new(expr))
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
                            })
                        });

                        Expression::MethodCall {
                            object: Box::new(expr),
                            method: "slice".to_string(),
                            type_args: None,
                            arguments: vec![
                                (None, Expression::Literal(Literal::Int(0))),
                                (None, *end_expr),
                            ],
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
                                })
                            });

                            Expression::MethodCall {
                                object: Box::new(expr),
                                method: "slice".to_string(),
                                type_args: None,
                                arguments: vec![(None, start_or_index), (None, *end_expr)],
                            }
                        } else {
                            // Regular index: [i]
                            self.expect(Token::RBracket)?;
                            Expression::Index {
                                object: Box::new(expr),
                                index: Box::new(start_or_index),
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
                    }
                }
                Token::As => {
                    self.advance();
                    let type_ = self.parse_type()?;
                    Expression::Cast {
                        expr: Box::new(expr),
                        type_,
                    }
                }
                Token::Bang => {
                    // Macro invocation: name!(...) or name![...] or name!{...}
                    if let Expression::Identifier(name) = expr {
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

    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
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

    // Helper: Extract a name from a pattern for backward compatibility
    fn pattern_to_name(pattern: &Pattern) -> String {
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

    // Helper: Convert a pattern to a string representation for enum bindings
    fn pattern_to_string(pattern: &Pattern) -> String {
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

    // Public wrapper methods for component compiler
    pub fn parse_expression_public(&mut self) -> Result<Expression, String> {
        self.parse_expression()
    }

    pub fn parse_type_public(&mut self) -> Result<Type, String> {
        self.parse_type()
    }

    pub fn parse_function_public(&mut self) -> Result<FunctionDecl, String> {
        self.parse_function()
    }
}
