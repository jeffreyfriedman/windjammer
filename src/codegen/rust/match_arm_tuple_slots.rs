//! A tuple literal in a match arm takes its slot types from a sibling arm: `Ok(p) => p, Err(_) => (0, "")` with
//! `p: (int, string)` makes the literal `(0, String)` (rustc needs the arms to agree).

use crate::parser::{Expression, MatchArm, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Slot types of the first non-literal arm whose body has a tuple type (`None` without a tuple-literal arm).
    pub(in crate::codegen::rust) fn sibling_arm_tuple_slots(
        &mut self,
        scrutinee: &Expression<'ast>,
        arms: &[MatchArm<'ast>],
    ) -> Option<Vec<Type>> {
        if !arms.iter().any(|a| matches!(a.body, Expression::Tuple { .. })) {
            return None;
        }
        for arm in arms.iter().filter(|a| !matches!(a.body, Expression::Tuple { .. })) {
            let bound = self.infer_match_bound_types(scrutinee, &arm.pattern);
            let saved: Vec<_> = bound.iter().map(|(n, _)| (n.clone(), self.local_var_types.get(n).cloned())).collect();
            for (n, t) in &bound {
                self.local_var_types.insert(n.clone(), t.clone());
            }
            let ty = self.infer_expression_type(arm.body);
            for (n, old) in saved {
                match old {
                    Some(t) => self.local_var_types.insert(n, t),
                    None => self.local_var_types.remove(&n),
                };
            }
            if let Some(Type::Tuple(slots)) = ty {
                return Some(slots);
            }
        }
        None
    }

    /// Hand the sibling slots to the arm's tuple literal (and to nothing else).
    pub(in crate::codegen::rust) fn offer_arm_tuple_slots(&mut self, body: &Expression<'ast>, sibling: &Option<Vec<Type>>) {
        if matches!(body, Expression::Tuple { .. }) {
            self.idiom.expected_tuple_slots = sibling.clone();
        }
    }
}
