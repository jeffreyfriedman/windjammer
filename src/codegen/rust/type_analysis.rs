//! Type analysis facade: [`TypeAnalyzer`] and standalone copy helpers; codegen inference lives in sibling modules.

pub use super::type_analysis_pure::{is_copy_type, is_known_copy_type};
pub use super::type_analyzer::TypeAnalyzer;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Type;

    #[test]
    fn test_is_copy_type_primitives() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.is_copy_type(&Type::Int));
        assert!(analyzer.is_copy_type(&Type::Bool));
        assert!(analyzer.is_copy_type(&Type::Float));
    }

    #[test]
    fn test_is_copy_type_non_copy() {
        let analyzer = TypeAnalyzer::new();
        assert!(!analyzer.is_copy_type(&Type::String));
    }

    #[test]
    fn test_is_partial_eq_type_primitives() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.is_partial_eq_type(&Type::Int));
        assert!(analyzer.is_partial_eq_type(&Type::Bool));
        assert!(analyzer.is_partial_eq_type(&Type::Float)); // Floats support PartialEq
        assert!(analyzer.is_partial_eq_type(&Type::String));
    }

    #[test]
    fn test_is_eq_type_no_floats() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.is_eq_type(&Type::Int));
        assert!(analyzer.is_eq_type(&Type::Bool));
        assert!(!analyzer.is_eq_type(&Type::Float)); // Floats don't support Eq
    }

    #[test]
    fn test_has_default() {
        let analyzer = TypeAnalyzer::new();
        assert!(analyzer.has_default(&Type::Int));
        assert!(analyzer.has_default(&Type::Bool));
        assert!(analyzer.has_default(&Type::String));
    }
}
