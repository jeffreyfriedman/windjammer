//! Rust `f32` / `f64` inherent method signatures for [`SignatureRegistry`].
//!
//! Single source of truth for primitive float method metadata — consumers derive
//! float-preservation and arg-coercion from registry lookup, not method-name lists.

use crate::parser::Type;

use super::{FunctionSignature, OwnershipMode, SignatureRegistry};

#[derive(Copy, Clone)]
enum ParamKind {
    Float,
    Int,
}

struct MethodDef {
    name: &'static str,
    params: &'static [ParamKind],
}

/// Rust `f32`/`f64` inherent methods that return the receiver float type.
const INHERENT_FLOAT_METHODS: &[MethodDef] = &[
    // Trigonometric / hyperbolic — no args
    MethodDef {
        name: "sin",
        params: &[],
    },
    MethodDef {
        name: "cos",
        params: &[],
    },
    MethodDef {
        name: "tan",
        params: &[],
    },
    MethodDef {
        name: "asin",
        params: &[],
    },
    MethodDef {
        name: "acos",
        params: &[],
    },
    MethodDef {
        name: "atan",
        params: &[],
    },
    MethodDef {
        name: "sinh",
        params: &[],
    },
    MethodDef {
        name: "cosh",
        params: &[],
    },
    MethodDef {
        name: "tanh",
        params: &[],
    },
    MethodDef {
        name: "asinh",
        params: &[],
    },
    MethodDef {
        name: "acosh",
        params: &[],
    },
    MethodDef {
        name: "atanh",
        params: &[],
    },
    // Exponential / logarithmic
    MethodDef {
        name: "exp",
        params: &[],
    },
    MethodDef {
        name: "exp2",
        params: &[],
    },
    MethodDef {
        name: "exp_m1",
        params: &[],
    },
    MethodDef {
        name: "ln",
        params: &[],
    },
    MethodDef {
        name: "log",
        params: &[],
    },
    MethodDef {
        name: "log2",
        params: &[],
    },
    MethodDef {
        name: "log10",
        params: &[],
    },
    MethodDef {
        name: "ln_1p",
        params: &[],
    },
    // Roots / power
    MethodDef {
        name: "sqrt",
        params: &[],
    },
    MethodDef {
        name: "cbrt",
        params: &[],
    },
    MethodDef {
        name: "recip",
        params: &[],
    },
    MethodDef {
        name: "powf",
        params: &[ParamKind::Float],
    },
    MethodDef {
        name: "powi",
        params: &[ParamKind::Int],
    },
    // Rounding
    MethodDef {
        name: "floor",
        params: &[],
    },
    MethodDef {
        name: "ceil",
        params: &[],
    },
    MethodDef {
        name: "round",
        params: &[],
    },
    MethodDef {
        name: "trunc",
        params: &[],
    },
    MethodDef {
        name: "fract",
        params: &[],
    },
    // Abs / sign
    MethodDef {
        name: "abs",
        params: &[],
    },
    MethodDef {
        name: "signum",
        params: &[],
    },
    MethodDef {
        name: "copysign",
        params: &[ParamKind::Float],
    },
    // Min / max / clamp
    MethodDef {
        name: "max",
        params: &[ParamKind::Float],
    },
    MethodDef {
        name: "min",
        params: &[ParamKind::Float],
    },
    MethodDef {
        name: "clamp",
        params: &[ParamKind::Float, ParamKind::Float],
    },
    // Two-arg math
    MethodDef {
        name: "atan2",
        params: &[ParamKind::Float],
    },
    MethodDef {
        name: "hypot",
        params: &[ParamKind::Float],
    },
    MethodDef {
        name: "mul_add",
        params: &[ParamKind::Float, ParamKind::Float],
    },
    MethodDef {
        name: "fma",
        params: &[ParamKind::Float, ParamKind::Float],
    },
    // Conversions
    MethodDef {
        name: "to_degrees",
        params: &[],
    },
    MethodDef {
        name: "to_radians",
        params: &[],
    },
];

fn build_float_method_sig(float_name: &str, def: &MethodDef) -> FunctionSignature {
    let float_ty = Type::Custom(float_name.to_string());
    let mut param_types = vec![float_ty.clone()];
    let mut param_ownership = vec![OwnershipMode::Borrowed]; // &self for primitives

    for kind in def.params {
        match kind {
            ParamKind::Float => {
                param_types.push(float_ty.clone());
                param_ownership.push(OwnershipMode::Owned);
            }
            ParamKind::Int => {
                param_types.push(Type::Custom("i32".to_string()));
                param_ownership.push(OwnershipMode::Owned);
            }
        }
    }

    FunctionSignature {
        name: format!("{float_name}::{}", def.name),
        param_types: param_types.clone(),
        formal_param_types: param_types,
        param_ownership,
        return_type: Some(float_ty),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: None,
        forwarding_borrow_params: None,
    }
}

/// Register `f32::method` and `f64::method` inherent signatures on `registry`.
pub fn register_primitive_float_signatures(registry: &mut SignatureRegistry) {
    for float_name in ["f32", "f64"] {
        for def in INHERENT_FLOAT_METHODS {
            let sig = build_float_method_sig(float_name, def);
            registry.add_function(sig.name.clone(), sig);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::stdlib_method_traits::{
        method_float_args_match_receiver, method_preserves_float_receiver,
    };

    #[test]
    fn f32_sin_preserves_receiver() {
        let reg = SignatureRegistry::stdlib();
        let ty = Type::Custom("f32".into());
        assert!(method_preserves_float_receiver("sin", Some(&ty), reg));
        assert!(!method_float_args_match_receiver("sin", Some(&ty), reg));
    }

    #[test]
    fn f32_max_has_float_arg() {
        let reg = SignatureRegistry::stdlib();
        let ty = Type::Custom("f32".into());
        assert!(method_preserves_float_receiver("max", Some(&ty), reg));
        assert!(method_float_args_match_receiver("max", Some(&ty), reg));
    }

    #[test]
    fn non_float_receiver_rejects_max() {
        let reg = SignatureRegistry::stdlib();
        let ty = Type::Custom("Slider".into());
        assert!(!method_preserves_float_receiver("max", Some(&ty), reg));
    }

    #[test]
    fn f64_clamp_preserves_receiver() {
        let reg = SignatureRegistry::stdlib();
        let ty = Type::Custom("f64".into());
        assert!(method_preserves_float_receiver("clamp", Some(&ty), reg));
        assert!(method_float_args_match_receiver("clamp", Some(&ty), reg));
    }

    #[test]
    fn type_float_receiver_preserves_clamp() {
        let reg = SignatureRegistry::stdlib();
        assert!(method_preserves_float_receiver(
            "clamp",
            Some(&Type::Float),
            reg
        ));
    }
}
