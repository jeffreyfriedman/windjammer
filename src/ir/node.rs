//! IR node definitions.
//!
//! IR nodes wrap AST declarations with safety type information.
//! During Phase 1, `IrFunction` is constructed from `AnalyzedFunction`
//! as a lossless bridge. In later phases, the constraint solver will
//! populate IR nodes directly.

use crate::analyzer::{AnalyzedFunction, OwnershipMode};
use crate::ir::annotations::OptimizationHints;
use crate::ir::safety_type::{BaseType, OwnedType, Region, SafetyType};
use crate::parser::ast::types::Type;
use std::collections::{HashMap, HashSet};

/// An IR-level function — carries safety types for all parameters and locals.
#[derive(Debug, Clone)]
pub struct IrFunction {
    /// Function name (qualified if in an impl block).
    pub name: String,

    /// Safety type of each parameter, keyed by parameter name.
    pub param_types: HashMap<String, SafetyType>,

    /// Safety type of the return value.
    pub return_type: SafetyType,

    /// Which local variables are mutated (derived from analyzer).
    pub mutated_locals: HashSet<String>,

    /// Which parameters are mutated (derived from analyzer).
    pub mutated_params: HashSet<String>,

    /// Parameters eligible for &str optimization.
    pub str_ref_params: HashSet<String>,

    /// Optimization annotations (derived from analyzer phases 2-9).
    pub optimizations: OptimizationHints,
}

/// Convert a parser `Type` to an IR `BaseType`.
/// References/MutableReferences are stripped — ownership lives in `OwnedType`.
pub fn parser_type_to_base_type(ty: &Type) -> BaseType {
    match ty {
        Type::Int | Type::Int32 => BaseType::I32,
        Type::Uint => BaseType::U32,
        Type::Float => BaseType::F64,
        Type::Bool => BaseType::Bool,
        Type::String => BaseType::String,
        Type::Custom(name) => match name.as_str() {
            "i8" => BaseType::I8,
            "i16" => BaseType::I16,
            "i32" => BaseType::I32,
            "i64" => BaseType::I64,
            "i128" => BaseType::I128,
            "u8" => BaseType::U8,
            "u16" => BaseType::U16,
            "u32" => BaseType::U32,
            "u64" => BaseType::U64,
            "u128" => BaseType::U128,
            "f32" => BaseType::F32,
            "f64" => BaseType::F64,
            "bool" => BaseType::Bool,
            "char" => BaseType::Char,
            "isize" => BaseType::I64,
            "usize" => BaseType::U64,
            "String" | "string" => BaseType::String,
            "()" => BaseType::Unit,
            _ => BaseType::Custom(name.clone()),
        },
        Type::Generic(_) => BaseType::Inferred,
        Type::Parameterized(name, args) => {
            let base_name = name.rsplit("::").next().unwrap_or(name);
            match base_name {
                "Vec" if args.len() == 1 => {
                    BaseType::Vec(Box::new(parser_type_to_base_type(&args[0])))
                }
                "Option" if args.len() == 1 => {
                    BaseType::Option(Box::new(parser_type_to_base_type(&args[0])))
                }
                "Result" if args.len() == 2 => BaseType::Result(
                    Box::new(parser_type_to_base_type(&args[0])),
                    Box::new(parser_type_to_base_type(&args[1])),
                ),
                "HashMap" | "BTreeMap" if args.len() == 2 => BaseType::HashMap(
                    Box::new(parser_type_to_base_type(&args[0])),
                    Box::new(parser_type_to_base_type(&args[1])),
                ),
                _ => BaseType::Custom(name.clone()),
            }
        }
        Type::Associated(base, _assoc) => BaseType::Custom(base.clone()),
        Type::TraitObject(name) => BaseType::TraitObject(name.clone()),
        Type::ImplTrait(name) => BaseType::ImplTrait(name.clone()),
        Type::Option(inner) => BaseType::Option(Box::new(parser_type_to_base_type(inner))),
        Type::Result(ok, err) => BaseType::Result(
            Box::new(parser_type_to_base_type(ok)),
            Box::new(parser_type_to_base_type(err)),
        ),
        Type::Vec(inner) => BaseType::Vec(Box::new(parser_type_to_base_type(inner))),
        Type::Array(inner, size) => {
            BaseType::Array(Box::new(parser_type_to_base_type(inner)), *size)
        }
        Type::Reference(inner) | Type::MutableReference(inner) => parser_type_to_base_type(inner),
        Type::RawPointer { mutable, pointee } => BaseType::RawPointer {
            mutable: *mutable,
            inner: Box::new(parser_type_to_base_type(pointee)),
        },
        Type::Tuple(elems) => BaseType::Tuple(elems.iter().map(parser_type_to_base_type).collect()),
        Type::Infer => BaseType::Inferred,
        Type::FunctionPointer {
            params,
            return_type,
        } => BaseType::FunctionPointer {
            params: params.iter().map(parser_type_to_base_type).collect(),
            return_type: Box::new(
                return_type
                    .as_ref()
                    .map(|rt| parser_type_to_base_type(rt))
                    .unwrap_or(BaseType::Unit),
            ),
        },
    }
}

impl IrFunction {
    /// Bridge from the existing `AnalyzedFunction` — lossless conversion.
    /// Maps all analyzer data into typed IR representation.
    pub fn from_analyzed(analyzed: &AnalyzedFunction<'_>) -> Self {
        let name = analyzed.decl.name.to_string();

        let mut region_counter: u32 = 1;

        let inferred_types_by_index: HashMap<usize, &Type> = analyzed
            .decl
            .parameters
            .iter()
            .enumerate()
            .map(|(i, p)| (i, &p.type_))
            .collect();

        let inferred_type_overrides: HashMap<String, &Type> = analyzed
            .inferred_param_types
            .iter()
            .enumerate()
            .filter_map(|(i, ty)| {
                analyzed
                    .decl
                    .parameters
                    .get(i)
                    .map(|p| (p.name.clone(), ty))
            })
            .collect();

        let param_types = analyzed
            .inferred_ownership
            .iter()
            .map(|(param_name, mode)| {
                let ownership = match mode {
                    OwnershipMode::Owned => OwnedType::Owned,
                    OwnershipMode::Borrowed => {
                        let r = Region::fresh(region_counter);
                        region_counter += 1;
                        OwnedType::Ref(r)
                    }
                    OwnershipMode::MutBorrowed => {
                        let r = Region::fresh(region_counter);
                        region_counter += 1;
                        OwnedType::MutRef(r)
                    }
                };

                let base = if let Some(ty) = inferred_type_overrides.get(param_name) {
                    parser_type_to_base_type(ty)
                } else {
                    let idx = analyzed
                        .decl
                        .parameters
                        .iter()
                        .position(|p| p.name == param_name.as_str());
                    if let Some(ty) = idx.and_then(|i| inferred_types_by_index.get(&i)) {
                        parser_type_to_base_type(ty)
                    } else {
                        BaseType::Inferred
                    }
                };

                let safety = SafetyType {
                    base,
                    ownership,
                    effects: crate::ir::safety_type::EffectSet::pure(),
                    taint: crate::ir::safety_type::TaintStatus::Clean,
                    const_eval: crate::ir::safety_type::ConstEval::Runtime,
                    exec_mode: None,
                };
                (param_name.clone(), safety)
            })
            .collect();

        let return_base = analyzed
            .decl
            .return_type
            .as_ref()
            .map(|rt| parser_type_to_base_type(rt))
            .unwrap_or(BaseType::Unit);
        let return_type = SafetyType::owned(return_base);

        let optimizations = OptimizationHints::from_analyzed(analyzed);

        IrFunction {
            name,
            param_types,
            return_type,
            mutated_locals: analyzed.mutated_variables.clone(),
            mutated_params: analyzed.mutated_parameters.clone(),
            str_ref_params: analyzed.str_ref_optimizable_params.clone(),
            optimizations,
        }
    }
}

/// A single IR node in the function body (future use).
/// During Phase 1 the body is not represented in the IR — codegen still reads
/// the AST directly. Phase 2+ will add expression-level IR nodes.
#[derive(Debug, Clone)]
pub struct IrNode {
    pub kind: IrNodeKind,
    pub safety_type: SafetyType,
}

/// IR node kinds (will expand significantly in Phase 2+).
#[derive(Debug, Clone)]
pub enum IrNodeKind {
    /// Placeholder for the current AST-direct codegen path.
    AstPassthrough,
    /// A call expression with resolved signature.
    Call { callee: String, args: Vec<IrNode> },
    /// A local variable binding.
    Let { name: String, mutable: bool },
    /// A field access.
    FieldAccess { base: Box<IrNode>, field: String },
}
