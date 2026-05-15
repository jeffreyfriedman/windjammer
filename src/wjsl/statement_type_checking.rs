//! Statement-level parsing and checking for WJSL function bodies (let, return, var, control-flow skip).

use crate::wjsl::ast::*;
use crate::wjsl::lexer::Token;
use crate::wjsl::shader_type_rules::{type_to_string, types_match};
use crate::wjsl::type_checker::BodyParser;
use anyhow::{anyhow, Result};

impl<'a> BodyParser<'a> {
    pub(crate) fn error_at(&self, msg: String) -> anyhow::Error {
        anyhow!(
            "[line {}:{}] {}",
            self.current_line,
            self.current_column,
            msg
        )
    }

    pub(crate) fn advance(&mut self) -> Token {
        self.current_line = self.lexer.line();
        self.current_column = self.lexer.column();
        std::mem::replace(&mut self.current, self.lexer.next_token())
    }

    pub(crate) fn parse_and_check(&mut self, return_type: Option<&ReturnType>) -> Result<()> {
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

    pub(crate) fn parse_type_annotation(&mut self) -> Result<Type> {
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

    pub(crate) fn skip_optional_angle_bracket(&mut self) {
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

    pub(crate) fn expect_ident(&mut self) -> Result<String> {
        if let Token::Ident(s) = &self.current {
            let name = s.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.error_at(format!("Expected identifier, found {:?}", self.current)))
        }
    }

    pub(crate) fn expect(&mut self, expected: Token) -> Result<()> {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at(format!("Expected {:?}, found {:?}", expected, self.current)))
        }
    }

    pub(crate) fn expect_semicolon(&mut self) -> Result<()> {
        if matches!(self.current, Token::Semicolon) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_at(format!("Expected semicolon, found {:?}", self.current)))
        }
    }
}
