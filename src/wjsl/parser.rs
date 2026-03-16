//! WJSL Parser - Converts .wjsl source to AST
//!
//! Parses RFC syntax: @vertex, @fragment, @compute, @group, @binding, structs, etc.

use crate::wjsl::ast::*;
use crate::wjsl::lexer::{Lexer, Token};
use anyhow::{anyhow, Result};

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token();
        Self {
            lexer,
            current: current,
            peeked: None,
        }
    }

    fn advance(&mut self) -> Token {
        let prev = std::mem::replace(
            &mut self.current,
            self.peeked.take().unwrap_or_else(|| self.lexer.next_token()),
        );
        prev
    }

    fn peek(&mut self) -> &Token {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token());
        }
        self.peeked.as_ref().unwrap()
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, found {:?}", expected, self.current))
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        if let Token::Ident(s) = &self.current {
            let name = s.clone();
            self.advance();
            Ok(name)
        } else {
            Err(anyhow!("Expected identifier, found {:?}", self.current))
        }
    }

    /// Parse scalar type from generic params - handles both keywords (u32, f32) and Ident
    fn parse_scalar_type_in_generic(&mut self) -> Result<ScalarType> {
        let st = match &self.current {
            Token::F32 => {
                self.advance();
                ScalarType::F32
            }
            Token::F64 => {
                self.advance();
                ScalarType::F64
            }
            Token::U32 => {
                self.advance();
                ScalarType::U32
            }
            Token::I32 => {
                self.advance();
                ScalarType::I32
            }
            Token::Bool => {
                self.advance();
                ScalarType::Bool
            }
            Token::Ident(s) => {
                let st = match s.as_str() {
                    "f32" => ScalarType::F32,
                    "f64" => ScalarType::F64,
                    "u32" => ScalarType::U32,
                    "i32" => ScalarType::I32,
                    "bool" => ScalarType::Bool,
                    _ => return Err(anyhow!("Invalid scalar type in generic: {}", s)),
                };
                self.advance();
                st
            }
            _ => return Err(anyhow!("Expected scalar type, found {:?}", self.current)),
        };
        Ok(st)
    }

    fn is_eof(&self) -> bool {
        matches!(self.current, Token::Eof)
    }

    /// Parse @group(N) @binding(N) - returns (group, binding)
    fn parse_binding_attrs(&mut self) -> Result<(u32, u32)> {
        let mut group = 0u32;
        let mut binding = 0u32;

        loop {
            match &self.current {
                Token::AtGroup => {
                    self.advance();
                    self.expect(Token::LParen)?;
                    if let Token::IntLiteral(n) = self.current {
                        group = n as u32;
                        self.advance();
                    } else {
                        return Err(anyhow!("Expected group number"));
                    }
                    self.expect(Token::RParen)?;
                }
                Token::AtBinding => {
                    self.advance();
                    self.expect(Token::LParen)?;
                    if let Token::IntLiteral(n) = self.current {
                        binding = n as u32;
                        self.advance();
                    } else {
                        return Err(anyhow!("Expected binding number"));
                    }
                    self.expect(Token::RParen)?;
                }
                _ => break,
            }
        }

        Ok((group, binding))
    }

    fn parse_type(&mut self) -> Result<Type> {
        let ty = match &self.current {
            Token::F32 => {
                self.advance();
                Type::Scalar(ScalarType::F32)
            }
            Token::F64 => {
                self.advance();
                Type::Scalar(ScalarType::F64)
            }
            Token::U32 => {
                self.advance();
                Type::Scalar(ScalarType::U32)
            }
            Token::I32 => {
                self.advance();
                Type::Scalar(ScalarType::I32)
            }
            Token::Bool => {
                self.advance();
                Type::Scalar(ScalarType::Bool)
            }
            Token::Vec2 => {
                self.advance();
                let elem = if matches!(self.current, Token::LAngle) {
                    self.advance();
                    let st = self.parse_scalar_type_in_generic()?;
                    self.expect(Token::RAngle)?;
                    Some(st)
                } else {
                    None
                };
                Type::Vec2(elem)
            }
            Token::Vec3 => {
                self.advance();
                let elem = if matches!(self.current, Token::LAngle) {
                    self.advance();
                    let st = self.parse_scalar_type_in_generic()?;
                    self.expect(Token::RAngle)?;
                    Some(st)
                } else {
                    None
                };
                Type::Vec3(elem)
            }
            Token::Vec4 => {
                self.advance();
                let elem = if matches!(self.current, Token::LAngle) {
                    self.advance();
                    let st = self.parse_scalar_type_in_generic()?;
                    self.expect(Token::RAngle)?;
                    Some(st)
                } else {
                    None
                };
                Type::Vec4(elem)
            }
            Token::Mat2x2 => {
                self.advance();
                let elem = if matches!(self.current, Token::LAngle) {
                    self.advance();
                    let st = self.parse_scalar_type_in_generic()?;
                    self.expect(Token::RAngle)?;
                    Some(st)
                } else {
                    None
                };
                Type::Mat2x2(elem)
            }
            Token::Mat3x3 => {
                self.advance();
                let elem = if matches!(self.current, Token::LAngle) {
                    self.advance();
                    let st = self.parse_scalar_type_in_generic()?;
                    self.expect(Token::RAngle)?;
                    Some(st)
                } else {
                    None
                };
                Type::Mat3x3(elem)
            }
            Token::Mat4x4 => {
                self.advance();
                let elem = if matches!(self.current, Token::LAngle) {
                    self.advance();
                    let st = self.parse_scalar_type_in_generic()?;
                    self.expect(Token::RAngle)?;
                    Some(st)
                } else {
                    None
                };
                Type::Mat4x4(elem)
            }
            Token::Array => {
                self.advance();
                self.expect(Token::LAngle)?;
                let inner = self.parse_type()?;
                self.expect(Token::RAngle)?;
                Type::Array(Box::new(inner))
            }
            Token::Texture2dType | Token::Texture2d => {
                self.advance();
                self.expect(Token::LAngle)?;
                let st = self.parse_scalar_type_in_generic()?;
                self.expect(Token::RAngle)?;
                Type::Texture2D(st)
            }
            Token::TextureCubeType => {
                self.advance();
                self.expect(Token::LAngle)?;
                let st = self.parse_scalar_type_in_generic()?;
                self.expect(Token::RAngle)?;
                Type::TextureCube(st)
            }
            Token::Texture3dType => {
                self.advance();
                self.expect(Token::LAngle)?;
                let st = self.parse_scalar_type_in_generic()?;
                self.expect(Token::RAngle)?;
                Type::Texture3D(st)
            }
            Token::SamplerType | Token::Sampler => {
                self.advance();
                Type::Sampler
            }
            Token::SamplerComparisonType => {
                self.advance();
                Type::SamplerComparison
            }
            Token::Ident(name) => {
                let n = name.clone();
                self.advance();
                Type::Struct(n)
            }
            _ => return Err(anyhow!("Expected type, found {:?}", self.current)),
        };
        Ok(ty)
    }

    fn parse_struct(&mut self) -> Result<StructDecl> {
        self.expect(Token::Struct)?;
        let name = self.expect_ident()?;
        self.expect(Token::LBrace)?;

        let mut fields = Vec::new();
        while !matches!(self.current, Token::RBrace | Token::Eof) {
            // Optional @align(N) or @size(N)
            let mut align = None;
            let mut size = None;
            while matches!(self.current, Token::AtAlign | Token::AtSize) {
                if matches!(self.current, Token::AtAlign) {
                    self.advance();
                    self.expect(Token::LParen)?;
                    if let Token::IntLiteral(n) = self.current {
                        align = Some(n as u32);
                        self.advance();
                    }
                    self.expect(Token::RParen)?;
                } else {
                    self.advance();
                    self.expect(Token::LParen)?;
                    if let Token::IntLiteral(n) = self.current {
                        size = Some(n as u32);
                        self.advance();
                    }
                    self.expect(Token::RParen)?;
                }
            }

            let field_name = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            fields.push(StructField {
                name: field_name,
                ty,
                align,
                size,
            });

            // Optional trailing comma
            if matches!(self.current, Token::Comma) {
                self.advance();
            }
        }

        self.expect(Token::RBrace)?;
        Ok(StructDecl { name, fields })
    }

    fn parse_binding_fixed(&mut self) -> Result<Binding> {
        let (group, binding) = self.parse_binding_attrs()?;

        let (name, kind) = if matches!(self.current, Token::Uniform) {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            self.expect(Token::Semicolon)?;
            (name, BindingKind::Uniform(ty))
        } else if matches!(self.current, Token::Storage) {
            self.advance();
            let access = if matches!(self.current, Token::ReadWrite) {
                self.advance();
                StorageAccess::ReadWrite
            } else if matches!(self.current, Token::Read) {
                self.advance();
                if matches!(self.current, Token::Write) {
                    self.advance();
                    StorageAccess::ReadWrite
                } else {
                    StorageAccess::Read
                }
            } else if matches!(self.current, Token::Write) {
                self.advance();
                StorageAccess::Write
            } else {
                StorageAccess::ReadWrite
            };
            let name = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            self.expect(Token::Semicolon)?;
            (name, BindingKind::Storage { access_mode: access, ty })
        } else if matches!(self.current, Token::Texture2d) {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            let texture_type = match &ty {
                Type::Texture2D(s) => TextureType::Texture2D(*s),
                Type::TextureCube(s) => TextureType::TextureCube(*s),
                Type::Texture3D(s) => TextureType::Texture3D(*s),
                _ => return Err(anyhow!("Expected texture type")),
            };
            self.expect(Token::Semicolon)?;
            return Ok(Binding {
                group,
                binding,
                name,
                kind: BindingKind::Texture { texture_type },
            });
        } else if matches!(self.current, Token::Sampler) {
            self.advance();
            let name = self.expect_ident()?;
            self.expect(Token::Colon)?;
            // Accept both Token::Sampler and Token::SamplerType for "sampler" type
            if !matches!(self.current, Token::SamplerType | Token::Sampler) {
                return Err(anyhow!("Expected sampler type"));
            }
            self.advance();
            self.expect(Token::Semicolon)?;
            return Ok(Binding {
                group,
                binding,
                name,
                kind: BindingKind::Sampler,
            });
        } else {
            return Err(anyhow!(
                "Expected uniform, storage, texture_2d, or sampler, found {:?}",
                self.current
            ));
        };

        Ok(Binding {
            group,
            binding,
            name,
            kind,
        })
    }

    fn parse_param(&mut self) -> Result<Param> {
        let mut location = None;
        let mut builtin = None;

        while matches!(
            self.current,
            Token::AtLocation | Token::AtBuiltin
        ) {
            if matches!(self.current, Token::AtLocation) {
                self.advance();
                self.expect(Token::LParen)?;
                if let Token::IntLiteral(n) = self.current {
                    location = Some(n as u32);
                    self.advance();
                }
                self.expect(Token::RParen)?;
            } else {
                self.advance();
                self.expect(Token::LParen)?;
                if let Token::Ident(s) = &self.current {
                    builtin = Some(s.clone());
                    self.advance();
                }
                self.expect(Token::RParen)?;
            }
        }

        let name = self.expect_ident()?;
        self.expect(Token::Colon)?;
        let ty = self.parse_type()?;

        Ok(Param {
            name,
            ty,
            location,
            builtin,
        })
    }

    fn parse_return_type(&mut self) -> Result<ReturnType> {
        let mut location = None;
        let mut builtin = None;

        while matches!(
            self.current,
            Token::AtLocation | Token::AtBuiltin
        ) {
            if matches!(self.current, Token::AtLocation) {
                self.advance();
                self.expect(Token::LParen)?;
                if let Token::IntLiteral(n) = self.current {
                    location = Some(n as u32);
                    self.advance();
                }
                self.expect(Token::RParen)?;
            } else {
                self.advance();
                self.expect(Token::LParen)?;
                if let Token::Ident(s) = &self.current {
                    builtin = Some(s.clone());
                    self.advance();
                }
                self.expect(Token::RParen)?;
            }
        }

        let ty = self.parse_type()?;
        Ok(ReturnType {
            ty,
            location,
            builtin,
        })
    }

    fn parse_function_params(&mut self) -> Result<Vec<Param>> {
        self.expect(Token::LParen)?;
        let mut params = Vec::new();

        while !matches!(self.current, Token::RParen | Token::Eof) {
            params.push(self.parse_param()?);
            if matches!(self.current, Token::Comma) {
                self.advance();
            }
        }

        self.expect(Token::RParen)?;
        Ok(params)
    }

    fn parse_entry_point(&mut self, stage: ShaderStage, workgroup_size: Option<(u32, u32, u32)>) -> Result<EntryPoint> {
        self.expect(Token::Fn)?;
        let name = self.expect_ident()?;
        let params = self.parse_function_params()?;

        let return_type = if matches!(self.current, Token::Arrow) {
            self.advance();
            Some(self.parse_return_type()?)
        } else {
            None
        };

        self.expect(Token::LBrace)?;
        let body_start = self.lexer.position();
        let mut depth = 1;

        while depth > 0 && !self.is_eof() {
            match &self.current {
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        let body_end = self.lexer.position() - 1;
                        let body = self.lexer.source()[body_start..body_end].trim();
                        self.advance();
                        return Ok(EntryPoint {
                            stage,
                            workgroup_size,
                            name,
                            params,
                            return_type,
                            body: body.to_string(),
                        });
                    }
                    self.advance();
                }
                Token::Eof => return Err(anyhow!("Unclosed function body")),
                _ => {
                    self.advance();
                }
            }
        }

        Err(anyhow!("Unclosed function body"))
    }

    fn parse_helper_function(&mut self) -> Result<Function> {
        self.expect(Token::Fn)?;
        let name = self.expect_ident()?;
        let params = self.parse_function_params()?;
        self.expect(Token::Arrow)?;
        let return_type = self.parse_type()?;

        self.expect(Token::LBrace)?;
        let body_start = self.lexer.position();
        let mut depth = 1;
        while depth > 0 && !self.is_eof() {
            match &self.current {
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        let body_end = self.lexer.position() - 1;
                        let body = self.lexer.source()[body_start..body_end].trim();
                        self.advance();
                        return Ok(Function {
                            name,
                            params,
                            return_type,
                            body: body.to_string(),
                        });
                    }
                    self.advance();
                }
                Token::Eof => return Err(anyhow!("Unclosed function body")),
                _ => {
                    self.advance();
                }
            }
        }

        Err(anyhow!("Unclosed function body"))
    }

    fn parse_module(&mut self) -> Result<ShaderModule> {
        let mut structs = Vec::new();
        let mut bindings = Vec::new();
        let mut functions = Vec::new();
        let mut entry_points = Vec::new();

        while !self.is_eof() {
            // Entry point attributes
            if matches!(self.current, Token::AtVertex) {
                self.advance();
                let ep = self.parse_entry_point(ShaderStage::Vertex, None)?;
                entry_points.push(ep);
                continue;
            }
            if matches!(self.current, Token::AtFragment) {
                self.advance();
                let ep = self.parse_entry_point(ShaderStage::Fragment, None)?;
                entry_points.push(ep);
                continue;
            }
            if matches!(self.current, Token::AtCompute) {
                self.advance();
                let workgroup = if matches!(self.current, Token::AtWorkgroupSize) {
                    self.advance(); // @workgroup_size
                    self.expect(Token::LParen)?;
                    let x = if let Token::IntLiteral(n) = self.current {
                        self.advance();
                        n as u32
                    } else {
                        return Err(anyhow!("Expected workgroup x"));
                    };
                    self.expect(Token::Comma)?;
                    let y = if let Token::IntLiteral(n) = self.current {
                        self.advance();
                        n as u32
                    } else {
                        return Err(anyhow!("Expected workgroup y"));
                    };
                    let z = if matches!(self.peek(), Token::RParen) {
                        1u32
                    } else {
                        self.expect(Token::Comma)?;
                        if let Token::IntLiteral(n) = self.current {
                            self.advance();
                            n as u32
                        } else {
                            return Err(anyhow!("Expected workgroup z"));
                        }
                    };
                    self.expect(Token::RParen)?;
                    Some((x, y, z))
                } else {
                    None
                };
                let ep = self.parse_entry_point(ShaderStage::Compute, workgroup)?;
                entry_points.push(ep);
                continue;
            }

            // Binding attributes @group @binding
            if matches!(self.current, Token::AtGroup) {
                let binding = self.parse_binding_fixed()?;
                bindings.push(binding);
                continue;
            }

            // Struct
            if matches!(self.current, Token::Struct) {
                let s = self.parse_struct()?;
                structs.push(s);
                continue;
            }

            // Helper function (fn without @vertex/@fragment/@compute)
            if matches!(self.current, Token::Fn) {
                // Peek - is next token an ident? If so, it's a helper. But we need to
                // distinguish from entry point. Entry points have @vertex etc BEFORE fn.
                // So if we're at Fn and we didn't just see @vertex, it's a helper.
                let func = self.parse_helper_function()?;
                functions.push(func);
                continue;
            }

            self.advance(); // Skip unknown
        }

        Ok(ShaderModule {
            structs,
            bindings,
            functions,
            entry_points,
        })
    }
}

/// Parse WJSL source into AST
pub fn parse_wjsl(source: &str) -> Result<ShaderModule> {
    let mut parser = Parser::new(source);
    parser.parse_module()
}
