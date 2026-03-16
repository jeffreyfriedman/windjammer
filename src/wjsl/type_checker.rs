//! WJSL Type Checker - Compile-time validation
//!
//! Catches shader type errors before codegen: binary ops, bindings, function signatures.

use crate::wjsl::ast::*;
use crate::wjsl::lexer::{Lexer, Token};
use crate::wjsl::parser::parse_wjsl;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Type check WJSL source. Returns Ok(()) if valid, Err with message if invalid.
pub fn type_check_wjsl(source: &str) -> Result<()> {
    let ast = parse_wjsl(source)?;
    check(&ast, source)
}

/// Type check a parsed shader module
pub fn check(ast: &ShaderModule, _source: &str) -> Result<()> {
    let mut checker = TypeChecker::new(ast);
    checker.check_bindings_unique()?;
    checker.check_entry_points()?;
    checker.check_functions()?;
    Ok(())
}

/// Type checker with symbol table
struct TypeChecker<'a> {
    ast: &'a ShaderModule,
    /// (group, binding) -> binding name for duplicate detection
    binding_slots: HashMap<(u32, u32), String>,
    /// Struct name -> StructDecl for field lookup
    structs: HashMap<String, &'a StructDecl>,
}

impl<'a> TypeChecker<'a> {
    fn new(ast: &'a ShaderModule) -> Self {
        let structs = ast
            .structs
            .iter()
            .map(|s| (s.name.clone(), s))
            .collect::<HashMap<_, _>>();
        Self {
            ast,
            binding_slots: HashMap::new(),
            structs,
        }
    }

    fn check_bindings_unique(&mut self) -> Result<()> {
        for b in &self.ast.bindings {
            let key = (b.group, b.binding);
            if let Some(prev) = self.binding_slots.get(&key) {
                return Err(anyhow!(
                    "Duplicate @binding({}) in @group({}): '{}' conflicts with '{}'",
                    b.binding,
                    b.group,
                    b.name,
                    prev
                ));
            }
            self.binding_slots.insert(key, b.name.clone());
        }
        Ok(())
    }

    fn check_entry_points(&self) -> Result<()> {
        for ep in &self.ast.entry_points {
            self.check_function_body(
                &ep.body,
                &ep.params,
                ep.return_type.as_ref(),
                &ep.name,
                true,
            )?;
        }
        Ok(())
    }

    fn check_functions(&self) -> Result<()> {
        for f in &self.ast.functions {
            self.check_function_body(
                &f.body,
                &f.params,
                f.return_type.as_ref().map(|t| ReturnType {
                    ty: t.clone(),
                    location: None,
                    builtin: None,
                }),
                &f.name,
                false,
            )?;
        }
        Ok(())
    }

    fn check_function_body(
        &self,
        body: &str,
        params: &[Param],
        return_type: Option<&ReturnType>,
        _fn_name: &str,
        _is_entry: bool,
    ) -> Result<()> {
        let mut symbols = HashMap::new();
        for p in params {
            symbols.insert(p.name.clone(), p.ty.clone());
        }
        for b in &self.ast.bindings {
            let ty = match &b.kind {
                BindingKind::Uniform(t) => t.clone(),
                BindingKind::Storage { ty, .. } => ty.clone(),
                BindingKind::Texture { texture_type } => match texture_type {
                    TextureType::Texture2D(s) => Type::Texture2D(*s),
                    TextureType::TextureCube(s) => Type::TextureCube(*s),
                    TextureType::Texture3D(s) => Type::Texture3D(*s),
                },
                BindingKind::Sampler => Type::Sampler,
            };
            symbols.insert(b.name.clone(), ty);
        }
        for pv in &self.ast.private_vars {
            symbols.insert(pv.name.clone(), pv.ty.clone());
        }

        let mut parser = BodyParser::new(body, symbols, self.structs.clone());
        parser.parse_and_check(return_type)
    }
}

/// Parser for function body - extracts statements and type-checks expressions
struct BodyParser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    symbols: HashMap<String, Type>,
    structs: HashMap<String, &'a StructDecl>,
}

impl<'a> BodyParser<'a> {
    fn new(
        body: &'a str,
        symbols: HashMap<String, Type>,
        structs: HashMap<String, &'a StructDecl>,
    ) -> Self {
        let mut lexer = Lexer::new(body);
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            symbols,
            structs,
        }
    }

    fn advance(&mut self) -> Token {
        std::mem::replace(&mut self.current, self.lexer.next_token())
    }

    fn parse_and_check(&mut self, return_type: Option<&ReturnType>) -> Result<()> {
        while !matches!(self.current, Token::Eof) {
            if matches!(self.current, Token::Let) {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(Token::Assign)?;
                let ty = self.parse_expr()?;
                self.symbols.insert(name, ty);
                self.expect_semicolon()?;
            } else if matches!(self.current, Token::Return) {
                self.advance();
                if !matches!(self.current, Token::Semicolon) {
                    let expr_ty = self.parse_expr()?;
                    if let Some(rt) = return_type {
                        if !types_match(&expr_ty, &rt.ty) {
                            return Err(anyhow!(
                                "Return type mismatch: expected {}, got {}",
                                type_to_string(&rt.ty),
                                type_to_string(&expr_ty)
                            ));
                        }
                    }
                }
                self.expect_semicolon()?;
            } else if matches!(self.current, Token::Var) {
                self.skip_var_decl()?;
            } else if matches!(self.current, Token::If) {
                self.skip_block()?;
            } else if matches!(self.current, Token::For) {
                self.skip_block()?;
            } else if matches!(self.current, Token::While) {
                self.skip_block()?;
            } else if matches!(self.current, Token::Loop) {
                self.skip_block()?;
            } else if matches!(self.current, Token::Switch) {
                self.skip_block()?;
            } else if matches!(self.current, Token::LBrace) {
                self.skip_braces()?;
            } else if matches!(self.current, Token::Semicolon) {
                self.advance();
            } else if matches!(self.current, Token::Eof) {
                break;
            } else {
                self.advance();
            }
        }
        Ok(())
    }

    fn skip_var_decl(&mut self) -> Result<()> {
        self.advance();
        while !matches!(self.current, Token::Semicolon | Token::Eof) {
            self.advance();
        }
        if matches!(self.current, Token::Semicolon) {
            self.advance();
        }
        Ok(())
    }

    fn skip_block(&mut self) -> Result<()> {
        while !matches!(self.current, Token::LBrace) && !matches!(self.current, Token::Semicolon)
            && !matches!(self.current, Token::Eof)
        {
            self.advance();
        }
        if matches!(self.current, Token::Semicolon) {
            self.advance();
            return Ok(());
        }
        if matches!(self.current, Token::Eof) {
            return Err(anyhow!("Expected block or statement"));
        }
        let mut depth = 0;
        loop {
            match &self.current {
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                }
                Token::Eof => return Err(anyhow!("Unclosed block")),
                _ => {
                    self.advance();
                }
            }
        }
        Ok(())
    }

    fn skip_braces(&mut self) -> Result<()> {
        self.expect(Token::LBrace)?;
        let mut depth = 1;
        while depth > 0 && !matches!(self.current, Token::Eof) {
            match &self.current {
                Token::LBrace => depth += 1,
                Token::RBrace => depth -= 1,
                _ => {}
            }
            self.advance();
        }
        Ok(())
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

    fn expect(&mut self, expected: Token) -> Result<()> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, found {:?}", expected, self.current))
        }
    }

    fn expect_semicolon(&mut self) -> Result<()> {
        if matches!(self.current, Token::Semicolon) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected semicolon, found {:?}", self.current))
        }
    }

    fn parse_expr(&mut self) -> Result<Type> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<Type> {
        let mut left = self.parse_multiplicative()?;
        loop {
            match &self.current {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = check_binary_op(&left, BinaryOp::Add, &right)?;
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = check_binary_op(&left, BinaryOp::Sub, &right)?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Type> {
        let mut left = self.parse_unary()?;
        loop {
            match &self.current {
                Token::Star => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = check_binary_op(&left, BinaryOp::Mul, &right)?;
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = check_binary_op(&left, BinaryOp::Div, &right)?;
                }
                Token::Percent => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = check_binary_op(&left, BinaryOp::Mod, &right)?;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Type> {
        if matches!(self.current, Token::Minus) {
            self.advance();
            let ty = self.parse_unary()?;
            if !is_numeric(&ty) {
                return Err(anyhow!("Cannot negate non-numeric type {}", type_to_string(&ty)));
            }
            Ok(ty)
        } else if matches!(self.current, Token::Not) {
            self.advance();
            let ty = self.parse_unary()?;
            if !matches!(ty, Type::Scalar(ScalarType::Bool)) {
                return Err(anyhow!("Cannot apply ! to non-bool type"));
            }
            Ok(ty)
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Type> {
        match &self.current {
            Token::FloatLiteral(_) => {
                self.advance();
                Ok(Type::Scalar(ScalarType::F32))
            }
            Token::IntLiteral(_) => {
                self.advance();
                Ok(Type::Scalar(ScalarType::U32))
            }
            Token::True | Token::False => {
                self.advance();
                Ok(Type::Scalar(ScalarType::Bool))
            }
            Token::Vec2 => {
                self.advance();
                self.parse_vec_constructor(2)
            }
            Token::Vec3 => {
                self.advance();
                self.parse_vec_constructor(3)
            }
            Token::Vec4 => {
                self.advance();
                self.parse_vec_constructor(4)
            }
            Token::Mat4x4 | Token::Mat3x3 | Token::Mat2x2 => {
                let mat = std::mem::replace(&mut self.current, Token::Eof);
                self.advance();
                self.expect(Token::LParen)?;
                let _ = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let (n, _) = match mat {
                    Token::Mat2x2 => (2, Type::Mat2x2(Some(ScalarType::F32))),
                    Token::Mat3x3 => (3, Type::Mat3x3(Some(ScalarType::F32))),
                    Token::Mat4x4 => (4, Type::Mat4x4(Some(ScalarType::F32))),
                    _ => (4, Type::Mat4x4(Some(ScalarType::F32))),
                };
                Ok(Type::Mat4x4(Some(ScalarType::F32)))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                if matches!(self.current, Token::LParen) {
                    self.parse_function_call(&name)
                } else if matches!(self.current, Token::Dot) {
                    self.advance();
                    let member = self.expect_ident()?;
                    if let Some(ty) = self.symbols.get(&name) {
                        self.get_swizzle_or_field_type(ty, &member)
                    } else {
                        Err(anyhow!("Unknown identifier '{}'", name))
                    }
                } else if let Some(ty) = self.symbols.get(&name) {
                    Ok(ty.clone())
                } else {
                    Err(anyhow!("Unknown identifier '{}'", name))
                }
            }
            Token::LParen => {
                self.advance();
                let ty = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(ty)
            }
            Token::LBracket => {
                self.advance();
                let _index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                if let Some(ty) = self.symbols.values().next() {
                    Ok(ty.clone())
                } else {
                    Ok(Type::Scalar(ScalarType::F32))
                }
            }
            _ => Err(anyhow!("Unexpected token in expression: {:?}", self.current)),
        }
    }

    fn get_swizzle_or_field_type(&self, ty: &Type, member: &str) -> Result<Type> {
        match ty {
            Type::Vec2(_) => match member {
                "x" | "y" | "r" | "g" => Ok(Type::Scalar(ScalarType::F32)),
                "xy" | "rg" => Ok(Type::Vec2(Some(ScalarType::F32))),
                _ => Ok(Type::Scalar(ScalarType::F32)),
            },
            Type::Vec3(_) => match member {
                "x" | "y" | "z" | "r" | "g" | "b" => Ok(Type::Scalar(ScalarType::F32)),
                "xy" | "xz" | "yz" | "rgb" => Ok(Type::Vec3(Some(ScalarType::F32))),
                "xyz" => Ok(Type::Vec3(Some(ScalarType::F32))),
                _ => Ok(Type::Scalar(ScalarType::F32)),
            },
            Type::Vec4(_) => match member {
                "x" | "y" | "z" | "w" | "r" | "g" | "b" | "a" => Ok(Type::Scalar(ScalarType::F32)),
                "xy" | "xz" | "xw" | "yz" | "yw" | "zw" => Ok(Type::Vec2(Some(ScalarType::F32))),
                "xyz" | "rgb" => Ok(Type::Vec3(Some(ScalarType::F32))),
                "xyzw" | "rgba" => Ok(Type::Vec4(Some(ScalarType::F32))),
                _ => Ok(Type::Scalar(ScalarType::F32)),
            },
            Type::Struct(struct_name) => {
                if let Some(s) = self.structs.get(struct_name) {
                    for f in &s.fields {
                        if f.name == member {
                            return Ok(f.ty.clone());
                        }
                    }
                }
                Err(anyhow!("Unknown field '{}'", member))
            }
            _ => Ok(Type::Scalar(ScalarType::F32)),
        }
    }

    fn parse_vec_constructor(&mut self, n: usize) -> Result<Type> {
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        loop {
            args.push(self.parse_expr()?);
            if matches!(self.current, Token::RParen) {
                break;
            }
            self.expect(Token::Comma)?;
        }
        self.expect(Token::RParen)?;
        let elem_type = scalar_of(&args[0]);
        Ok(match n {
            2 => Type::Vec2(Some(elem_type)),
            3 => Type::Vec3(Some(elem_type)),
            4 => Type::Vec4(Some(elem_type)),
            _ => Type::Vec4(Some(elem_type)),
        })
    }

    fn parse_function_call(&mut self, _name: &str) -> Result<Type> {
        self.expect(Token::LParen)?;
        while !matches!(self.current, Token::RParen | Token::Eof) {
            let _ = self.parse_expr()?;
            if matches!(self.current, Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;
        Ok(Type::Vec4(Some(ScalarType::F32)))
    }
}

#[derive(Copy, Clone)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

fn check_binary_op(left: &Type, op: BinaryOp, right: &Type) -> Result<Type> {
    match op {
        BinaryOp::Add | BinaryOp::Sub => {
            if vec_scalar_mismatch(left, right) {
                let op_str = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "subtract",
                    _ => "op",
                };
                return Err(anyhow!(
                    "Cannot {} {} to {}. Did you mean {} + vec3({}, {}, {})?",
                    op_str,
                    type_to_string(right),
                    type_to_string(left),
                    type_to_string(left),
                    type_to_string(right),
                    type_to_string(right),
                    type_to_string(right)
                ));
            }
            if !same_vec_size(left, right) {
                return Err(anyhow!(
                    "Cannot {} {} and {} - vector sizes must match",
                    match op {
                        BinaryOp::Add => "add",
                        BinaryOp::Sub => "subtract",
                        _ => "op",
                    },
                    type_to_string(left),
                    type_to_string(right)
                ));
            }
            Ok(left.clone())
        }
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            if op == BinaryOp::Mod && !is_integer_scalar(right) {
                return Err(anyhow!("Modulo requires integer types"));
            }
            if is_scalar(left) && is_vector(right) {
                Ok(right.clone())
            } else if is_vector(left) && is_scalar(right) {
                Ok(left.clone())
            } else if is_matrix(left) && is_vector(right) {
                if let (Type::Mat4x4(_), Type::Vec4(_)) = (left, right) {
                    Ok(Type::Vec4(Some(ScalarType::F32)))
                } else if let (Type::Mat3x3(_), Type::Vec3(_)) = (left, right) {
                    Ok(Type::Vec3(Some(ScalarType::F32)))
                } else {
                    Err(anyhow!(
                        "Matrix * vector: mat4x4 * vec4 or mat3x3 * vec3 required"
                    ))
                }
            } else if is_vector(left) && is_vector(right) {
                if !same_vec_size(left, right) {
                    return Err(anyhow!(
                        "Cannot multiply {} and {} - vector sizes must match",
                        type_to_string(left),
                        type_to_string(right)
                    ));
                }
                Ok(left.clone())
            } else if is_scalar(left) && is_scalar(right) {
                Ok(left.clone())
            } else {
                Err(anyhow!(
                    "Invalid operands for *: {} and {}",
                    type_to_string(left),
                    type_to_string(right)
                ))
            }
        }
    }
}

fn vec_scalar_mismatch(left: &Type, right: &Type) -> bool {
    (is_vector(left) && is_scalar(right)) || (is_scalar(left) && is_vector(right))
}

fn same_vec_size(left: &Type, right: &Type) -> bool {
    vec_size(left) == vec_size(right)
}

fn vec_size(ty: &Type) -> Option<usize> {
    match ty {
        Type::Vec2(_) => Some(2),
        Type::Vec3(_) => Some(3),
        Type::Vec4(_) => Some(4),
        _ => None,
    }
}

fn is_scalar(ty: &Type) -> bool {
    matches!(ty, Type::Scalar(_))
}

fn is_vector(ty: &Type) -> bool {
    matches!(ty, Type::Vec2(_) | Type::Vec3(_) | Type::Vec4(_))
}

fn is_matrix(ty: &Type) -> bool {
    matches!(ty, Type::Mat2x2(_) | Type::Mat3x3(_) | Type::Mat4x4(_))
}

fn is_numeric(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Scalar(ScalarType::F32)
            | Type::Scalar(ScalarType::F64)
            | Type::Scalar(ScalarType::U32)
            | Type::Scalar(ScalarType::I32)
    ) || is_vector(ty)
        || is_matrix(ty)
}

fn is_integer_scalar(ty: &Type) -> bool {
    matches!(ty, Type::Scalar(ScalarType::U32) | Type::Scalar(ScalarType::I32))
}

fn scalar_of(ty: &Type) -> ScalarType {
    match ty {
        Type::Scalar(s) => *s,
        Type::Vec2(Some(s)) | Type::Vec3(Some(s)) | Type::Vec4(Some(s)) => *s,
        _ => ScalarType::F32,
    }
}

fn types_match(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Scalar(s1), Type::Scalar(s2)) => s1 == s2,
        (Type::Vec2(e1), Type::Vec2(e2)) => e1 == e2,
        (Type::Vec3(e1), Type::Vec3(e2)) => e1 == e2,
        (Type::Vec4(e1), Type::Vec4(e2)) => e1 == e2,
        (Type::Mat4x4(e1), Type::Mat4x4(e2)) => e1 == e2,
        (Type::Struct(n1), Type::Struct(n2)) => n1 == n2,
        _ => false,
    }
}

fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Scalar(ScalarType::F32) => "f32".to_string(),
        Type::Scalar(ScalarType::F64) => "f64".to_string(),
        Type::Scalar(ScalarType::U32) => "u32".to_string(),
        Type::Scalar(ScalarType::I32) => "i32".to_string(),
        Type::Scalar(ScalarType::Bool) => "bool".to_string(),
        Type::Vec2(e) => format!("vec2<{}>", scalar_str(e.unwrap_or(ScalarType::F32))),
        Type::Vec3(e) => format!("vec3<{}>", scalar_str(e.unwrap_or(ScalarType::F32))),
        Type::Vec4(e) => format!("vec4<{}>", scalar_str(e.unwrap_or(ScalarType::F32))),
        Type::Mat4x4(_) => "mat4x4".to_string(),
        Type::Struct(n) => n.clone(),
        _ => "unknown".to_string(),
    }
}

fn scalar_str(s: ScalarType) -> &'static str {
    match s {
        ScalarType::F32 => "f32",
        ScalarType::F64 => "f64",
        ScalarType::U32 => "u32",
        ScalarType::I32 => "i32",
        ScalarType::Bool => "bool",
    }
}
