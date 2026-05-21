//! WGSL-oriented type rules for WJSL: binary ops, indexing, structural equality.

use crate::wjsl::ast::*;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::wjsl) enum BinaryOp {
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

pub(in crate::wjsl) fn check_binary_op(left: &Type, op: BinaryOp, right: &Type) -> Result<Type> {
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
            } else if is_matrix(left) && is_matrix(right) {
                if let (Type::Mat4x4(_), Type::Mat4x4(_)) = (left, right) {
                    Ok(Type::Mat4x4(Some(ScalarType::F32)))
                } else if let (Type::Mat3x3(_), Type::Mat3x3(_)) = (left, right) {
                    Ok(Type::Mat3x3(Some(ScalarType::F32)))
                } else if let (Type::Mat2x2(_), Type::Mat2x2(_)) = (left, right) {
                    Ok(Type::Mat2x2(Some(ScalarType::F32)))
                } else {
                    Err(anyhow!(
                        "Matrix * matrix: operand sizes must match (e.g. mat4x4 * mat4x4)"
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

pub(in crate::wjsl) fn is_numeric(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Scalar(ScalarType::F32)
            | Type::Scalar(ScalarType::F64)
            | Type::Scalar(ScalarType::U32)
            | Type::Scalar(ScalarType::I32)
    ) || is_vector(ty)
        || is_matrix(ty)
}

pub(in crate::wjsl) fn is_integer_scalar(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Scalar(ScalarType::U32) | Type::Scalar(ScalarType::I32)
    )
}

/// Element type when indexing: array[i] -> element, vec4[i] -> f32, mat4x4[i] -> vec4
pub(in crate::wjsl) fn element_type_for_index(ty: &Type) -> Result<Type> {
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

pub(in crate::wjsl) fn scalar_of(ty: &Type) -> ScalarType {
    match ty {
        Type::Scalar(s) => *s,
        Type::Vec2(Some(s)) | Type::Vec3(Some(s)) | Type::Vec4(Some(s)) => *s,
        _ => ScalarType::F32,
    }
}

fn normalize_scalar(e: &Option<ScalarType>) -> ScalarType {
    e.unwrap_or(ScalarType::F32)
}

pub(in crate::wjsl) fn types_match(a: &Type, b: &Type) -> bool {
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

pub(in crate::wjsl) fn type_to_string(ty: &Type) -> String {
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
