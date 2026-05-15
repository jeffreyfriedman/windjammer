//! Defer-drop opportunities for large owned parameters with small returns.

use crate::parser::*;

use super::super::{
    Analyzer, DeferDropOptimization, DeferDropReason, EstimatedSize, OwnershipMode,
    SignatureRegistry,
};

impl<'ast> Analyzer<'ast> {
    pub(in crate::analyzer) fn detect_defer_drop_opportunities(
        &self,
        func: &FunctionDecl,
        registry: &SignatureRegistry,
    ) -> Vec<DeferDropOptimization> {
        let mut optimizations = Vec::new();

        for param in &func.parameters {
            let ownership = match param.ownership {
                OwnershipHint::Ref => OwnershipMode::Borrowed,
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Owned => OwnershipMode::Owned,
                OwnershipHint::Inferred => self
                    .infer_parameter_ownership(
                        &param.name,
                        &param.type_,
                        &func.body,
                        &func.return_type,
                        registry,
                        &func.name,
                        func,
                    )
                    .unwrap_or(OwnershipMode::Owned),
            };

            if ownership == OwnershipMode::Owned {
                let param_size = self.estimate_type_size(&param.type_);

                if matches!(param_size, EstimatedSize::Large | EstimatedSize::VeryLarge) {
                    if let Some(ref ret_type) = func.return_type {
                        if self.is_small_type(ret_type) && self.is_safe_to_defer(&param.type_) {
                            optimizations.push(DeferDropOptimization {
                                variable: param.name.clone(),
                                estimated_size: param_size,
                                reason: DeferDropReason::LargeOwnedParameter,
                                location: func.body.len().saturating_sub(1),
                            });
                        }
                    }
                }
            }
        }

        optimizations
    }

    pub(crate) fn estimate_type_size(&self, ty: &Type) -> EstimatedSize {
        match ty {
            Type::Parameterized(name, _)
                if crate::type_classification::is_large_collection(name) =>
            {
                EstimatedSize::Large
            }
            Type::Parameterized(name, _)
                if crate::type_classification::is_medium_collection(name) =>
            {
                EstimatedSize::Medium
            }
            Type::Vec(_) => EstimatedSize::Medium,
            Type::String => EstimatedSize::Medium,
            Type::Custom(_) => EstimatedSize::Medium,
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => EstimatedSize::Small,
            Type::Reference(_) | Type::MutableReference(_) => EstimatedSize::Small,
            _ => EstimatedSize::Small,
        }
    }

    pub(crate) fn is_small_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
        ) || matches!(ty, Type::Custom(name) if name == "usize" || name == "isize")
            || matches!(ty, Type::Reference(_) | Type::MutableReference(_))
    }

    pub(crate) fn is_safe_to_defer(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) | Type::Parameterized(name, _) => {
                if crate::type_classification::has_significant_drop(name) {
                    return false;
                }

                if crate::type_classification::is_large_collection(name)
                    || crate::type_classification::is_medium_collection(name)
                    || name == "String"
                {
                    return true;
                }

                true
            }
            Type::Vec(_) | Type::String => true,
            _ => false,
        }
    }
}
