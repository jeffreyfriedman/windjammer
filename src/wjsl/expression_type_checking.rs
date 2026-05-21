//! Expression type validation in WJSL function bodies.

use crate::wjsl::ast::*;
use crate::wjsl::lexer::Token;
use crate::wjsl::shader_type_rules::{
    check_binary_op, element_type_for_index, is_integer_scalar, is_numeric, scalar_of,
    type_to_string, BinaryOp,
};
use crate::wjsl::type_checker::BodyParser;
use anyhow::{anyhow, Result};

impl<'a> BodyParser<'a> {
    pub(crate) fn parse_expr(&mut self) -> Result<Type> {
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
                // NOTE: Removed skip_optional_angle_bracket() check here because it causes
                // the parser to skip tokens when < is a comparison operator, not a generic.
                // Generic type parameters should be handled explicitly in type contexts only.
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
            _ => {
                Err(anyhow!(
                    "[line {}:{}] Unexpected token in expression: {:?}",
                    self.current_line,
                    self.current_column,
                    self.current
                ))
            }
        }
    }

    pub(crate) fn get_swizzle_or_field_type(&self, ty: &Type, member: &str) -> Result<Type> {
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
