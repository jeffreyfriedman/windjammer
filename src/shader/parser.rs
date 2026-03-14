//! Parser for .wjsl (Windjammer Shader Language)

use crate::shader::ast::{
    AccessMode, ScalarType, ShaderModule, StorageDecl, Type, UniformDecl,
};
use anyhow::{anyhow, Result};
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum Token {
    Ident(String),
    Number(u32), // Used for @workgroup_size(8, 8)
    Colon,
    LAngle,
    RAngle,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Shader,
    Uniform,
    Storage,
    Read,
    Write,
    At,
    Eof,
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn next(&mut self) -> Option<char> {
        self.chars.next()
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.next();
                }
                Some('/') => {
                    self.next();
                    if self.peek() == Some('/') {
                        self.next();
                        while self.peek().map(|c| c != '\n').unwrap_or(false) {
                            self.next();
                        }
                    } else {
                        return;
                    }
                }
                _ => break,
            }
        }
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.next();
            } else {
                break;
            }
        }
        s
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let c = match self.next() {
            Some(c) => c,
            None => return Token::Eof,
        };

        match c {
            ':' => Token::Colon,
            '<' => Token::LAngle,
            '>' => Token::RAngle,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '@' => Token::At,
            c if c.is_alphabetic() || c == '_' => {
                let mut s = String::from(c);
                s.push_str(&self.read_ident());
                match s.as_str() {
                    "shader" => Token::Shader,
                    "uniform" => Token::Uniform,
                    "storage" => Token::Storage,
                    "read" => Token::Read,
                    "write" => Token::Write,
                    "Vec2" => Token::Ident("Vec2".to_string()),
                    "Vec3" => Token::Ident("Vec3".to_string()),
                    "Vec4" => Token::Ident("Vec4".to_string()),
                    "Mat4" => Token::Ident("Mat4".to_string()),
                    "array" => Token::Ident("array".to_string()),
                    "f32" => Token::Ident("f32".to_string()),
                    "f64" => Token::Ident("f64".to_string()),
                    "u32" => Token::Ident("u32".to_string()),
                    "i32" => Token::Ident("i32".to_string()),
                    "bool" => Token::Ident("bool".to_string()),
                    _ => Token::Ident(s),
                }
            }
            _ => Token::Eof,
        }
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token();
        Self { lexer, current }
    }

    fn advance(&mut self) -> Token {
        let prev = std::mem::replace(&mut self.current, self.lexer.next_token());
        prev
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        if self.current == expected {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, found {:?}", expected, self.current))
        }
    }

    fn parse_scalar_type(&mut self) -> Result<ScalarType> {
        if let Token::Ident(s) = &self.current {
            let st = match s.as_str() {
                "f32" => ScalarType::F32,
                "f64" => ScalarType::F64,
                "u32" => ScalarType::U32,
                "i32" => ScalarType::I32,
                "bool" => ScalarType::Bool,
                _ => return Err(anyhow!("Unknown scalar type: {}", s)),
            };
            self.advance();
            Ok(st)
        } else {
            Err(anyhow!("Expected scalar type, found {:?}", self.current))
        }
    }

    fn parse_type(&mut self) -> Result<Type> {
        match &self.current {
            Token::Ident(s) => match s.as_str() {
                "Vec2" => {
                    self.advance();
                    self.expect(Token::LAngle)?;
                    let st = self.parse_scalar_type()?;
                    self.expect(Token::RAngle)?;
                    Ok(Type::Vec2(st))
                }
                "Vec3" => {
                    self.advance();
                    self.expect(Token::LAngle)?;
                    let st = self.parse_scalar_type()?;
                    self.expect(Token::RAngle)?;
                    Ok(Type::Vec3(st))
                }
                "Vec4" => {
                    self.advance();
                    self.expect(Token::LAngle)?;
                    let st = self.parse_scalar_type()?;
                    self.expect(Token::RAngle)?;
                    Ok(Type::Vec4(st))
                }
                "Mat4" => {
                    self.advance();
                    Ok(Type::Mat4)
                }
                "array" => {
                    self.advance();
                    self.expect(Token::LAngle)?;
                    let inner = self.parse_type()?;
                    self.expect(Token::RAngle)?;
                    Ok(Type::Array(Box::new(inner)))
                }
                "f32" | "f64" | "u32" | "i32" | "bool" => {
                    let st = self.parse_scalar_type()?;
                    Ok(Type::Scalar(st))
                }
                _ => {
                    let name = s.clone();
                    self.advance();
                    Ok(Type::Struct(name))
                }
            },
            _ => Err(anyhow!("Expected type, found {:?}", self.current)),
        }
    }

    fn parse_shader(&mut self) -> Result<ShaderModule> {
        self.expect(Token::Shader)?;

        let name = if let Token::Ident(s) = &self.current {
            s.clone()
        } else {
            return Err(anyhow!("Expected shader name, found {:?}", self.current));
        };
        self.advance();

        self.expect(Token::LBrace)?;

        let mut uniforms = Vec::new();
        let mut storage = Vec::new();
        let mut binding = 0u32;

        loop {
            self.lexer.skip_whitespace_and_comments();
            if self.current == Token::RBrace || self.current == Token::Eof {
                break;
            }

            match &self.current {
                Token::Uniform => {
                    self.advance();
                    let decl_name = if let Token::Ident(s) = &self.current {
                        s.clone()
                    } else {
                        return Err(anyhow!("Expected uniform name"));
                    };
                    self.advance();
                    self.expect(Token::Colon)?;
                    let ty = self.parse_type()?;
                    uniforms.push(UniformDecl {
                        name: decl_name,
                        ty,
                        binding,
                    });
                    binding += 1;
                }
                Token::Storage => {
                    self.advance();
                    let decl_name = if let Token::Ident(s) = &self.current {
                        s.clone()
                    } else {
                        return Err(anyhow!("Expected storage name"));
                    };
                    self.advance();
                    self.expect(Token::Colon)?;
                    let ty = self.parse_type()?;
                    let access = if self.current == Token::Read {
                        self.advance();
                        AccessMode::Read
                    } else if self.current == Token::Write {
                        self.advance();
                        AccessMode::Write
                    } else {
                        AccessMode::ReadWrite
                    };
                    storage.push(StorageDecl {
                        name: decl_name,
                        ty,
                        binding,
                        access,
                    });
                    binding += 1;
                }
                Token::At => {
                    // Skip @compute, @workgroup_size, fn - consume until next decl or }
                    self.advance();
                    while self.current != Token::RBrace && self.current != Token::Eof {
                        if self.current == Token::Uniform || self.current == Token::Storage {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }

        self.expect(Token::RBrace)?;

        Ok(ShaderModule {
            name,
            uniforms,
            storage,
            functions: Vec::new(),
        })
    }
}

/// Parse a .wjsl source string into a ShaderModule.
pub fn parse_shader(source: &str) -> Result<ShaderModule> {
    let mut parser = Parser::new(source);
    parser.parse_shader()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shader_uniforms() {
        let source = r#"
        shader MyShader {
            uniform screen_size: Vec2<f32>
            storage output: array<Vec4<f32>>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.name, "MyShader");
        assert_eq!(module.uniforms.len(), 1);
        assert_eq!(module.uniforms[0].name, "screen_size");
        assert_eq!(module.uniforms[0].ty, Type::Vec2(ScalarType::F32));
        assert_eq!(module.storage.len(), 1);
        assert_eq!(module.storage[0].name, "output");
        assert_eq!(
            module.storage[0].ty,
            Type::Array(Box::new(Type::Vec4(ScalarType::F32)))
        );
    }

    #[test]
    fn test_parse_shader_uniform_only() {
        let source = r#"
        shader S {
            uniform x: Vec2<f32>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.name, "S");
        assert_eq!(module.uniforms.len(), 1);
        assert_eq!(module.uniforms[0].name, "x");
        assert_eq!(module.uniforms[0].ty, Type::Vec2(ScalarType::F32));
    }

    #[test]
    fn test_parse_shader_multiple_uniforms() {
        let source = r#"
        shader S {
            uniform a: Vec2<f32>
            uniform b: Vec4<f32>
            uniform c: Mat4
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.uniforms.len(), 3);
        assert_eq!(module.uniforms[0].name, "a");
        assert_eq!(module.uniforms[1].name, "b");
        assert_eq!(module.uniforms[2].name, "c");
    }

    #[test]
    fn test_parse_shader_scalar_types() {
        let source = r#"
        shader S {
            uniform x: Vec2<f32>
            uniform y: Vec2<f64>
            uniform z: Vec2<u32>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.uniforms[0].ty, Type::Vec2(ScalarType::F32));
        assert_eq!(module.uniforms[1].ty, Type::Vec2(ScalarType::F64));
        assert_eq!(module.uniforms[2].ty, Type::Vec2(ScalarType::U32));
    }

    #[test]
    fn test_parse_shader_array_types() {
        let source = r#"
        shader S {
            storage output: array<Vec4<f32>>
            storage nodes: array<SvoNode>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(
            module.storage[0].ty,
            Type::Array(Box::new(Type::Vec4(ScalarType::F32)))
        );
        assert_eq!(
            module.storage[1].ty,
            Type::Array(Box::new(Type::Struct("SvoNode".to_string())))
        );
    }

    #[test]
    fn test_parse_shader_vec3_vec4() {
        let source = r#"
        shader S {
            uniform pos: Vec3<f32>
            uniform color: Vec4<f32>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.uniforms[0].ty, Type::Vec3(ScalarType::F32));
        assert_eq!(module.uniforms[1].ty, Type::Vec4(ScalarType::F32));
    }

    #[test]
    fn test_parse_shader_binding_order() {
        let source = r#"
        shader S {
            uniform a: Vec2<f32>
            storage b: array<Vec4<f32>>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.uniforms[0].binding, 0);
        assert_eq!(module.storage[0].binding, 1);
    }

    #[test]
    fn test_parse_shader_with_comments() {
        let source = r#"
        // Comment
        shader S {
            uniform x: Vec2<f32>  // screen size
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.uniforms.len(), 1);
    }

    #[test]
    fn test_parse_shader_empty() {
        let source = r#"
        shader Empty {
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.name, "Empty");
        assert!(module.uniforms.is_empty());
        assert!(module.storage.is_empty());
    }

    #[test]
    fn test_parse_shader_struct_type() {
        let source = r#"
        shader S {
            uniform camera: CameraData
            storage nodes: array<SvoNode>
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(
            module.uniforms[0].ty,
            Type::Struct("CameraData".to_string())
        );
        assert_eq!(
            module.storage[0].ty,
            Type::Array(Box::new(Type::Struct("SvoNode".to_string())))
        );
    }

    #[test]
    fn test_parse_shader_mat4() {
        let source = r#"
        shader S {
            uniform view_proj: Mat4
        }
        "#;
        let module = parse_shader(source).unwrap();
        assert_eq!(module.uniforms[0].ty, Type::Mat4);
    }
}
