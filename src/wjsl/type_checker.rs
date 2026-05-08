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
            let return_type_wrapper = f.return_type.as_ref().map(|t| ReturnType {
                ty: t.clone(),
                location: None,
                builtin: None,
            });
            self.check_function_body(
                &f.body,
                &f.params,
                return_type_wrapper.as_ref(),
                &f.name,
                false,
            )?;
        }
        Ok(())
    }

    fn collect_function_signatures(&self) -> Vec<(String, Type)> {
        self.ast
            .functions
            .iter()
            .filter_map(|f| {
                f.return_type
                    .as_ref()
                    .map(|rt| (f.name.clone(), rt.clone()))
            })
            .collect()
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
        for cd in &self.ast.const_decls {
            if let Some(ref ty) = cd.ty {
                symbols.insert(cd.name.clone(), ty.clone());
            }
        }

        let fn_sigs = self.collect_function_signatures();
        let mut parser = BodyParser::new(body, symbols, self.structs.clone(), fn_sigs);
        parser.parse_and_check(return_type)
    }
}

/// Parser for function body - extracts statements and type-checks expressions
struct BodyParser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    current_line: usize,
    current_column: usize,
    symbols: HashMap<String, Type>,
    structs: HashMap<String, &'a StructDecl>,
    ast_functions: Vec<(String, Type)>,
}

impl<'a> BodyParser<'a> {
    fn new(
        body: &'a str,
        symbols: HashMap<String, Type>,
        structs: HashMap<String, &'a StructDecl>,
        ast_functions: Vec<(String, Type)>,
    ) -> Self {
        let mut lexer = Lexer::new(body);
        let line = lexer.line();
        let column = lexer.column();
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            current_line: line,
            current_column: column,
            symbols,
            structs,
            ast_functions,
        }
    }

    fn error_at(&self, msg: String) -> anyhow::Error {
        anyhow!(
            "[line {}:{}] {}",
            self.current_line,
            self.current_column,
            msg
        )
    }

    fn advance(&mut self) -> Token {
        self.current_line = self.lexer.line();
        self.current_column = self.lexer.column();
        std::mem::replace(&mut self.current, self.lexer.next_token())
    }

    fn parse_and_check(&mut self, return_type: Option<&ReturnType>) -> Result<()> {
        while !matches!(self.current, Token::Eof) {
            if matches!(self.current, Token::Let) {
                self.advance();
                if matches!(self.current, Token::Mut) {
                    self.advance();
                    self.parse_var_decl_body()?;
                } else {
                    let name = self.expect_ident()?;

                    let explicit_ty = if matches!(self.current, Token::Colon) {
                        self.advance();
                        Some(self.parse_type_annotation()?)
                    } else {
                        None
                    };

                    let inferred_ty = if matches!(self.current, Token::Assign) {
                        self.advance();
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };

                    let ty = explicit_ty
                        .or(inferred_ty)
                        .unwrap_or(Type::Scalar(ScalarType::F32));
                    self.symbols.insert(name, ty);
                    self.expect_semicolon()?;
                }
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
                self.parse_var_decl()?;
            } else if matches!(
                self.current,
                Token::If | Token::For | Token::While | Token::Loop | Token::Switch
            ) {
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

    fn parse_var_decl(&mut self) -> Result<()> {
        self.advance(); // consume `var`
        self.parse_var_decl_body()
    }

    fn parse_var_decl_body(&mut self) -> Result<()> {
        let name = self.expect_ident()?;

        let explicit_ty = if matches!(self.current, Token::Colon) {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let inferred_ty = if matches!(self.current, Token::Assign) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        let ty = explicit_ty
            .or(inferred_ty)
            .unwrap_or(Type::Scalar(ScalarType::F32));
        self.symbols.insert(name, ty);
        self.expect_semicolon()?;
        Ok(())
    }

    fn parse_type_annotation(&mut self) -> Result<Type> {
        match &self.current {
            Token::F32 => {
                self.advance();
                Ok(Type::Scalar(ScalarType::F32))
            }
            Token::F64 => {
                self.advance();
                Ok(Type::Scalar(ScalarType::F64))
            }
            Token::U32 => {
                self.advance();
                Ok(Type::Scalar(ScalarType::U32))
            }
            Token::I32 => {
                self.advance();
                Ok(Type::Scalar(ScalarType::I32))
            }
            Token::Bool => {
                self.advance();
                Ok(Type::Scalar(ScalarType::Bool))
            }
            Token::Vec2 => {
                self.advance();
                self.skip_optional_angle_bracket();
                Ok(Type::Vec2(Some(ScalarType::F32)))
            }
            Token::Vec3 => {
                self.advance();
                self.skip_optional_angle_bracket();
                Ok(Type::Vec3(Some(ScalarType::F32)))
            }
            Token::Vec4 => {
                self.advance();
                self.skip_optional_angle_bracket();
                Ok(Type::Vec4(Some(ScalarType::F32)))
            }
            Token::Mat4x4 => {
                self.advance();
                self.skip_optional_angle_bracket();
                Ok(Type::Mat4x4(Some(ScalarType::F32)))
            }
            Token::Mat3x3 => {
                self.advance();
                self.skip_optional_angle_bracket();
                Ok(Type::Mat3x3(Some(ScalarType::F32)))
            }
            Token::Mat2x2 => {
                self.advance();
                self.skip_optional_angle_bracket();
                Ok(Type::Mat2x2(Some(ScalarType::F32)))
            }
            Token::Array => {
                self.advance();
                let mut elem_type = Type::Scalar(ScalarType::F32);
                if matches!(self.current, Token::LAngle) {
                    self.advance();
                    elem_type = self.parse_type_annotation()?;
                    if matches!(self.current, Token::Comma) {
                        self.advance();
                        while !matches!(self.current, Token::RAngle | Token::Shr | Token::Eof) {
                            self.advance();
                        }
                    }
                    if matches!(self.current, Token::Shr) {
                        self.current = Token::RAngle;
                    } else if matches!(self.current, Token::RAngle) {
                        self.advance();
                    }
                }
                Ok(Type::Array(Box::new(elem_type), None))
            }
            _ => {
                let name = self.expect_ident()?;
                Ok(Type::Struct(name))
            }
        }
    }

    fn skip_optional_angle_bracket(&mut self) {
        if matches!(self.current, Token::LAngle) {
            self.advance();
            while !matches!(self.current, Token::RAngle | Token::Shr | Token::Eof) {
                self.advance();
            }
            if matches!(self.current, Token::Shr) {
                self.current = Token::RAngle;
            } else if matches!(self.current, Token::RAngle) {
                self.advance();
            }
        }
    }

    fn skip_block(&mut self) -> Result<()> {
        while !matches!(self.current, Token::LBrace)
            && !matches!(self.current, Token::Semicolon)
            && !matches!(self.current, Token::Eof)
        {
            self.advance();
        }
        if matches!(self.current, Token::Semicolon) {
            self.advance();
            return Ok(());
        }
        if matches!(self.current, Token::Eof) {
            return Err(self.error_at("Expected block or statement".to_string()));
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
            Err(self.error_at(format!("Expected identifier, found {:?}", self.current)))
        }
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at(format!("Expected {:?}, found {:?}", expected, self.current)))
        }
    }

    fn expect_semicolon(&mut self) -> Result<()> {
        if matches!(self.current, Token::Semicolon) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at(format!("Expected semicolon, found {:?}", self.current)))
        }
    }

    fn parse_expr(&mut self) -> Result<Type> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Type> {
        let mut left = self.parse_logical_and()?;
        while matches!(self.current, Token::Or) {
            self.advance();
            let _right = self.parse_logical_and()?;
            left = Type::Scalar(ScalarType::Bool);
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Type> {
        let mut left = self.parse_comparison()?;
        while matches!(self.current, Token::And) {
            self.advance();
            let _right = self.parse_comparison()?;
            left = Type::Scalar(ScalarType::Bool);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Type> {
        let left = self.parse_bitwise_or()?;
        match &self.current {
            Token::EqEq | Token::Ne | Token::Le | Token::Ge | Token::LAngle | Token::RAngle => {
                self.advance();
                let _right = self.parse_bitwise_or()?;
                Ok(Type::Scalar(ScalarType::Bool))
            }
            _ => Ok(left),
        }
    }

    fn parse_bitwise_or(&mut self) -> Result<Type> {
        let mut left = self.parse_bitwise_xor()?;
        while let Token::BitOr = &self.current {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = check_binary_op(&left, BinaryOp::BitOr, &right)?;
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Type> {
        let mut left = self.parse_bitwise_and()?;
        while let Token::BitXor = &self.current {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = check_binary_op(&left, BinaryOp::BitXor, &right)?;
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Type> {
        let mut left = self.parse_shift()?;
        while let Token::BitAnd = &self.current {
            self.advance();
            let right = self.parse_shift()?;
            left = check_binary_op(&left, BinaryOp::BitAnd, &right)?;
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Type> {
        let mut left = self.parse_additive()?;
        loop {
            match &self.current {
                Token::Shl => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = check_binary_op(&left, BinaryOp::Shl, &right)?;
                }
                Token::Shr => {
                    self.advance();
                    let right = self.parse_additive()?;
                    left = check_binary_op(&left, BinaryOp::Shr, &right)?;
                }
                _ => break,
            }
        }
        Ok(left)
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
                return Err(anyhow!(
                    "Cannot negate non-numeric type {}",
                    type_to_string(&ty)
                ));
            }
            Ok(ty)
        } else if matches!(self.current, Token::Not) {
            self.advance();
            let ty = self.parse_unary()?;
            if !matches!(ty, Type::Scalar(ScalarType::Bool)) {
                return Err(anyhow!("Cannot apply ! to non-bool type"));
            }
            Ok(ty)
        } else if matches!(self.current, Token::BitAnd) {
            // Address-of: &expr (e.g. atomicAdd(&draw_count, n))
            self.advance();
            let ty = self.parse_unary()?;
            Ok(ty)
        } else if matches!(self.current, Token::BitNot) {
            self.advance();
            let ty = self.parse_unary()?;
            if !is_integer_scalar(&ty) {
                return Err(anyhow!(
                    "Bitwise NOT (~) requires integer type, got {}",
                    type_to_string(&ty)
                ));
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
            Token::F32 | Token::F64 | Token::U32 | Token::I32 | Token::Bool => {
                let scalar = match &self.current {
                    Token::F32 => ScalarType::F32,
                    Token::F64 => ScalarType::F64,
                    Token::U32 => ScalarType::U32,
                    Token::I32 => ScalarType::I32,
                    Token::Bool => ScalarType::Bool,
                    _ => unreachable!(),
                };
                self.advance();
                self.expect(Token::LParen)?;
                let _arg = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let mut ty = Type::Scalar(scalar);
                loop {
                    if matches!(self.current, Token::Dot) {
                        self.advance();
                        let member = self.expect_ident()?;
                        ty = self.get_swizzle_or_field_type(&ty, &member)?;
                    } else {
                        break;
                    }
                }
                Ok(ty)
            }
            Token::Mat4x4 | Token::Mat3x3 | Token::Mat2x2 => {
                let mat = std::mem::replace(&mut self.current, Token::Eof);
                self.advance();
                if matches!(self.current, Token::LAngle) {
                    self.advance();
                    while !matches!(self.current, Token::RAngle | Token::Eof) {
                        self.advance();
                    }
                    if matches!(self.current, Token::RAngle) {
                        self.advance();
                    }
                }
                self.expect(Token::LParen)?;
                loop {
                    let _ = self.parse_expr()?;
                    if matches!(self.current, Token::RParen) {
                        break;
                    }
                    self.expect(Token::Comma)?;
                }
                self.expect(Token::RParen)?;
                match mat {
                    Token::Mat2x2 => Ok(Type::Mat2x2(Some(ScalarType::F32))),
                    Token::Mat3x3 => Ok(Type::Mat3x3(Some(ScalarType::F32))),
                    Token::Mat4x4 => Ok(Type::Mat4x4(Some(ScalarType::F32))),
                    _ => Ok(Type::Mat4x4(Some(ScalarType::F32))),
                }
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                if matches!(self.current, Token::LParen) {
                    return self.parse_function_call(&name);
                }
                if name == "bitcast" && matches!(self.current, Token::LAngle) {
                    self.advance();
                    let target_ty = self.parse_type_annotation()?;
                    if matches!(self.current, Token::Shr) {
                        self.current = Token::RAngle;
                    } else if matches!(self.current, Token::RAngle) {
                        self.advance();
                    }
                    self.expect(Token::LParen)?;
                    let _arg = self.parse_expr()?;
                    self.expect(Token::RParen)?;
                    let mut result = target_ty;
                    loop {
                        if matches!(self.current, Token::Dot) {
                            self.advance();
                            let member = self.expect_ident()?;
                            result = self.get_swizzle_or_field_type(&result, &member)?;
                        } else if matches!(self.current, Token::LBracket) {
                            self.advance();
                            let _index = self.parse_expr()?;
                            self.expect(Token::RBracket)?;
                            result = element_type_for_index(&result)?;
                        } else {
                            break;
                        }
                    }
                    return Ok(result);
                }
                if matches!(self.current, Token::LAngle) {
                    self.skip_optional_angle_bracket();
                    if matches!(self.current, Token::LParen) {
                        return self.parse_function_call(&name);
                    }
                }
                let mut ty = self
                    .symbols
                    .get(&name)
                    .ok_or_else(|| anyhow!("Unknown identifier '{}'", name))?
                    .clone();
                // Postfix: [index] and .member can chain (e.g. arr[i], arr[i].x, m[0][3])
                loop {
                    if matches!(self.current, Token::LBracket) {
                        self.advance();
                        let _index = self.parse_expr()?;
                        self.expect(Token::RBracket)?;
                        ty = element_type_for_index(&ty)?;
                    } else if matches!(self.current, Token::Dot) {
                        self.advance();
                        let member = self.expect_ident()?;
                        ty = self.get_swizzle_or_field_type(&ty, &member)?;
                    } else {
                        break;
                    }
                }
                Ok(ty)
            }
            Token::LParen => {
                self.advance();
                let mut ty = self.parse_expr()?;
                self.expect(Token::RParen)?;
                loop {
                    if matches!(self.current, Token::Dot) {
                        self.advance();
                        let member = self.expect_ident()?;
                        ty = self.get_swizzle_or_field_type(&ty, &member)?;
                    } else if matches!(self.current, Token::LBracket) {
                        self.advance();
                        let _index = self.parse_expr()?;
                        self.expect(Token::RBracket)?;
                        ty = element_type_for_index(&ty)?;
                    } else {
                        break;
                    }
                }
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
            _ => Err(anyhow!(
                "Unexpected token in expression: {:?}",
                self.current
            )),
        }
    }

    fn get_swizzle_or_field_type(&self, ty: &Type, member: &str) -> Result<Type> {
        match ty {
            Type::Vec2(scalar) | Type::Vec3(scalar) | Type::Vec4(scalar) => {
                let scalar_ty = scalar.unwrap_or(ScalarType::F32);
                let max_component = match ty {
                    Type::Vec2(_) => 2,
                    Type::Vec3(_) => 3,
                    Type::Vec4(_) => 4,
                    _ => unreachable!(),
                };
                self.resolve_swizzle(member, max_component, scalar_ty)
            }
            Type::Struct(struct_name) => {
                if let Some(s) = self.structs.get(struct_name) {
                    for f in &s.fields {
                        if f.name == member {
                            return Ok(f.ty.clone());
                        }
                    }
                }
                Err(anyhow!(
                    "Unknown field '{}' on struct '{}'",
                    member,
                    struct_name
                ))
            }
            _ => Err(anyhow!(
                "Cannot access member '{}' on type {:?}",
                member,
                ty
            )),
        }
    }

    fn resolve_swizzle(
        &self,
        member: &str,
        max_components: usize,
        scalar: ScalarType,
    ) -> Result<Type> {
        let valid_xyzw = ['x', 'y', 'z', 'w'];
        let valid_rgba = ['r', 'g', 'b', 'a'];
        let chars: Vec<char> = member.chars().collect();
        if chars.is_empty() || chars.len() > 4 {
            return Err(anyhow!("Invalid swizzle '{}'", member));
        }
        let using_rgba =
            valid_rgba.contains(&chars[0]) && !valid_xyzw[..max_components].contains(&chars[0]);
        let valid_set = if using_rgba {
            &valid_rgba[..max_components]
        } else {
            &valid_xyzw[..max_components]
        };
        for &c in &chars {
            if !valid_set.contains(&c) {
                return Err(anyhow!(
                    "Invalid swizzle component '{}' in '{}' for vec{}",
                    c,
                    member,
                    max_components
                ));
            }
        }
        match chars.len() {
            1 => Ok(Type::Scalar(scalar)),
            2 => Ok(Type::Vec2(Some(scalar))),
            3 => Ok(Type::Vec3(Some(scalar))),
            4 => Ok(Type::Vec4(Some(scalar))),
            _ => unreachable!(),
        }
    }

    fn parse_vec_constructor(&mut self, n: usize) -> Result<Type> {
        if matches!(self.current, Token::LAngle) {
            self.advance();
            while !matches!(self.current, Token::RAngle | Token::Eof) {
                self.advance();
            }
            if matches!(self.current, Token::RAngle) {
                self.advance();
            }
        }
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

    fn parse_function_call(&mut self, name: &str) -> Result<Type> {
        self.expect(Token::LParen)?;
        let mut arg_types = Vec::new();
        while !matches!(self.current, Token::RParen | Token::Eof) {
            arg_types.push(self.parse_expr()?);
            if matches!(self.current, Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;

        let first_arg = arg_types
            .first()
            .cloned()
            .unwrap_or(Type::Scalar(ScalarType::F32));

        let ty = match name {
            "length" | "dot" | "distance" => Type::Scalar(ScalarType::F32),
            "any" | "all" => Type::Scalar(ScalarType::Bool),
            "arrayLength" => Type::Scalar(ScalarType::U32),
            "abs" | "ceil" | "floor" | "round" | "sign" | "sqrt" | "inverseSqrt" | "sin"
            | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" | "exp" | "exp2" | "log"
            | "log2" | "fract" | "saturate" | "normalize" | "clamp" | "mix" | "smoothstep"
            | "step" | "min" | "max" | "pow" | "select" | "trunc" => first_arg,
            "cross" => Type::Vec3(Some(ScalarType::F32)),
            "reflect" | "refract" | "faceForward" => first_arg,
            "transpose" => first_arg,
            "determinant" => Type::Scalar(ScalarType::F32),
            "textureSample" | "textureLoad" => Type::Vec4(Some(ScalarType::F32)),
            _ => {
                let mut matched = false;
                let mut user_fn_type = first_arg.clone();
                for f in &self.ast_functions {
                    if f.0 == name {
                        user_fn_type = f.1.clone();
                        matched = true;
                        break;
                    }
                }
                if matched {
                    user_fn_type
                } else {
                    first_arg
                }
            }
        };

        let mut result = ty;
        loop {
            if matches!(self.current, Token::Dot) {
                self.advance();
                let member = self.expect_ident()?;
                result = self.get_swizzle_or_field_type(&result, &member)?;
            } else if matches!(self.current, Token::LBracket) {
                self.advance();
                let _index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                result = element_type_for_index(&result)?;
            } else {
                break;
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

fn check_binary_op(left: &Type, op: BinaryOp, right: &Type) -> Result<Type> {
    match op {
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
            if !is_integer_scalar(left) || !is_integer_scalar(right) {
                return Err(anyhow!(
                    "Bitwise operator requires integer types, got {} and {}",
                    type_to_string(left),
                    type_to_string(right)
                ));
            }
            if !types_match(left, right) {
                return Err(anyhow!(
                    "Bitwise operator requires same integer type on both sides, got {} and {}",
                    type_to_string(left),
                    type_to_string(right)
                ));
            }
            Ok(left.clone())
        }
        BinaryOp::Add | BinaryOp::Sub => {
            if is_scalar(left) && is_vector(right) {
                return Ok(right.clone());
            }
            if is_vector(left) && is_scalar(right) {
                return Ok(left.clone());
            }
            if is_vector(left) && is_vector(right) && !same_vec_size(left, right) {
                let op_str = if op == BinaryOp::Add {
                    "add"
                } else {
                    "subtract"
                };
                return Err(anyhow!(
                    "Cannot {} {} and {} - vector sizes must match",
                    op_str,
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
    matches!(
        ty,
        Type::Scalar(ScalarType::U32) | Type::Scalar(ScalarType::I32)
    )
}

/// Element type when indexing: array[i] -> element, vec4[i] -> f32, mat4x4[i] -> vec4
fn element_type_for_index(ty: &Type) -> Result<Type> {
    match ty {
        Type::Array(inner, _) => Ok((**inner).clone()),
        Type::Vec2(e) => Ok(Type::Scalar((*e).unwrap_or(ScalarType::F32))),
        Type::Vec3(e) => Ok(Type::Scalar((*e).unwrap_or(ScalarType::F32))),
        Type::Vec4(e) => Ok(Type::Scalar((*e).unwrap_or(ScalarType::F32))),
        Type::Mat2x2(e) => Ok(Type::Vec2(*e)),
        Type::Mat3x3(e) => Ok(Type::Vec3(*e)),
        Type::Mat4x4(e) => Ok(Type::Vec4(*e)),
        _ => Err(anyhow!("Cannot index type {}", type_to_string(ty))),
    }
}

fn scalar_of(ty: &Type) -> ScalarType {
    match ty {
        Type::Scalar(s) => *s,
        Type::Vec2(Some(s)) | Type::Vec3(Some(s)) | Type::Vec4(Some(s)) => *s,
        _ => ScalarType::F32,
    }
}

fn normalize_scalar(e: &Option<ScalarType>) -> ScalarType {
    e.unwrap_or(ScalarType::F32)
}

fn types_match(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Scalar(s1), Type::Scalar(s2)) => s1 == s2,
        (Type::Vec2(e1), Type::Vec2(e2)) => normalize_scalar(e1) == normalize_scalar(e2),
        (Type::Vec3(e1), Type::Vec3(e2)) => normalize_scalar(e1) == normalize_scalar(e2),
        (Type::Vec4(e1), Type::Vec4(e2)) => normalize_scalar(e1) == normalize_scalar(e2),
        (Type::Mat2x2(e1), Type::Mat2x2(e2)) => normalize_scalar(e1) == normalize_scalar(e2),
        (Type::Mat3x3(e1), Type::Mat3x3(e2)) => normalize_scalar(e1) == normalize_scalar(e2),
        (Type::Mat4x4(e1), Type::Mat4x4(e2)) => normalize_scalar(e1) == normalize_scalar(e2),
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
        Type::Mat2x2(_) => "mat2x2".to_string(),
        Type::Mat3x3(_) => "mat3x3".to_string(),
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
