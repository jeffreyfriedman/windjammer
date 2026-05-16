// Trait parsing — extracted from item_parser.rs

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

fn type_structurally_contains_self(ty: &Type) -> bool {
    match ty {
        Type::Custom(name) if name == "Self" => true,
        Type::Associated(base, _) if base == "Self" => true,
        Type::Option(inner)
        | Type::Vec(inner)
        | Type::Reference(inner)
        | Type::MutableReference(inner) => type_structurally_contains_self(inner),
        Type::Result(ok, err) => {
            type_structurally_contains_self(ok) || type_structurally_contains_self(err)
        }
        Type::Tuple(types) => types.iter().any(type_structurally_contains_self),
        Type::Parameterized(_, args) => args.iter().any(type_structurally_contains_self),
        Type::Array(inner, _) => type_structurally_contains_self(inner),
        Type::FunctionPointer {
            params,
            return_type,
        } => {
            params.iter().any(type_structurally_contains_self)
                || return_type
                    .as_ref()
                    .is_some_and(|t| type_structurally_contains_self(t))
        }
        Type::RawPointer { pointee, .. } => type_structurally_contains_self(pointee),
        _ => false,
    }
}

impl Parser {
    pub(crate) fn parse_trait(&mut self) -> Result<TraitDecl<'static>, String> {
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

            // Capture all consecutive doc comments (/// or //!)
            let doc_comment = self.collect_doc_comments();

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

            // Abstract trait methods: only force by-value `self` when the return type mentions
            // `Self` (e.g. `fn into_inner(self) -> Self`). A bare `self` with a non-Self return
            // (e.g. `fn is_enabled(self) -> bool`) must stay `Inferred` so the analyzer emits
            // `&self` and `dyn Trait` / `Box<dyn Trait>` method calls compile.
            let mut parameters = parameters;
            if body.is_none()
                && parameters.len() == 1
                && parameters[0].name == "self"
                && parameters[0].ownership == OwnershipHint::Inferred
                && return_type
                    .as_ref()
                    .is_some_and(type_structurally_contains_self)
            {
                parameters[0].ownership = OwnershipHint::Owned;
            }

            // Non-Self returns with abstract trait methods: keep Inferred.
            // The analyzer will determine &self, &mut self, or self based on
            // the implementation bodies. This avoids object-safety issues
            // (bare `self` prevents dyn Trait usage).

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
}
