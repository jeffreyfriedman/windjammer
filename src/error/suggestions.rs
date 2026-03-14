//! Helpful fix suggestions for common compiler errors.
//!
//! Maps error codes to actionable suggestions that help developers
//! resolve issues quickly.

/// Suggest a fix for a given error code.
///
/// Returns `Some(suggestion)` if we have helpful advice for this error,
/// or `None` if we don't have a specific suggestion.
///
/// The `context` parameter can provide additional context (e.g., variable name)
/// for more tailored suggestions. It may be used in future enhancements.
#[allow(unused_variables)]
pub fn suggest_fix(error_code: &str, context: &str) -> Option<String> {
    match error_code {
        "E0425" | "WJ0001" => Some(
            "Variable not found. Did you mean one of these?\n  \
            - Check for typos\n  \
            - Ensure variable is in scope\n  \
            - Import the module if from another file"
                .to_string(),
        ),
        "E0308" | "WJ0003" => Some(
            "Type mismatch. Common fixes:\n  \
            - Add .clone() if moving owned value\n  \
            - Use &reference if borrowing\n  \
            - Check function return type matches usage"
                .to_string(),
        ),
        "E0404" | "WJ0004" => Some(
            "Expected type. Common fixes:\n  \
            - Add explicit type annotation\n  \
            - Check function signature matches call\n  \
            - Ensure generic parameters are specified"
                .to_string(),
        ),
        "E0583" | "WJ0005" => Some(
            "Scope/block error. Common fixes:\n  \
            - Check braces are balanced\n  \
            - Ensure variable is declared before use\n  \
            - Verify block structure"
                .to_string(),
        ),
        "E0061" | "WJ0006" => Some(
            "Argument count mismatch. Common fixes:\n  \
            - Check function expects correct number of arguments\n  \
            - Add missing arguments or remove extra ones\n  \
            - Verify argument types match"
                .to_string(),
        ),
        "E0063" | "WJ0007" => Some(
            "Missing struct field. Common fixes:\n  \
            - Add all required fields when constructing\n  \
            - Use ..Default::default() for optional fields\n  \
            - Check struct definition for required fields"
                .to_string(),
        ),
        "E0382" | "WJ0008" => Some(
            "Use of moved value. Common fixes:\n  \
            - Use .clone() to keep a copy\n  \
            - Pass by reference instead of owned\n  \
            - Restructure to avoid multiple uses"
                .to_string(),
        ),
        "E0596" | "WJ0009" => Some(
            "Borrowing error. Common fixes:\n  \
            - Add mut if you need to mutate\n  \
            - Use & for immutable borrow\n  \
            - Check that value is not already borrowed"
                .to_string(),
        ),
        "E0599" | "WJ0002" => Some(
            "Method/function not found. Common fixes:\n  \
            - Check spelling of method name\n  \
            - Ensure trait is in scope (use/import)\n  \
            - Verify type has this method"
                .to_string(),
        ),
        "E0601" => Some(
            "Shader/GPU type mismatch. Common fixes:\n  \
            - Ensure host type matches shader uniform type\n  \
            - Check Vec2<f32> vs Vec2<f64> (common mismatch)\n  \
            - Verify buffer binding slots match"
                .to_string(),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_fix_e0425() {
        let s = suggest_fix("E0425", "undefined_var");
        assert!(s.is_some());
        let s = s.unwrap();
        assert!(s.contains("Variable not found"));
        assert!(s.contains("typos"));
        assert!(s.contains("scope"));
    }

    #[test]
    fn test_suggest_fix_e0308() {
        let s = suggest_fix("E0308", "");
        assert!(s.is_some());
        let s = s.unwrap();
        assert!(s.contains("Type mismatch"));
        assert!(s.contains("clone"));
    }

    #[test]
    fn test_suggest_fix_wj0001() {
        let s = suggest_fix("WJ0001", "");
        assert!(s.is_some());
        assert!(suggest_fix("WJ0001", "").unwrap().contains("Variable not found"));
    }

    #[test]
    fn test_suggest_fix_e0404() {
        let s = suggest_fix("E0404", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("Expected type"));
    }

    #[test]
    fn test_suggest_fix_e0583() {
        let s = suggest_fix("E0583", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("Scope"));
    }

    #[test]
    fn test_suggest_fix_e0061() {
        let s = suggest_fix("E0061", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("Argument count"));
    }

    #[test]
    fn test_suggest_fix_e0063() {
        let s = suggest_fix("E0063", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("struct field"));
    }

    #[test]
    fn test_suggest_fix_e0382() {
        let s = suggest_fix("E0382", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("moved value"));
    }

    #[test]
    fn test_suggest_fix_e0596() {
        let s = suggest_fix("E0596", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("Borrowing"));
    }

    #[test]
    fn test_suggest_fix_e0599() {
        let s = suggest_fix("E0599", "");
        assert!(s.is_some());
        assert!(s.unwrap().contains("Method"));
    }

    #[test]
    fn test_suggest_fix_e0601() {
        let s = suggest_fix("E0601", "");
        assert!(s.is_some());
        let s = s.unwrap();
        assert!(s.contains("Shader"));
        assert!(s.contains("Vec2"));
    }

    #[test]
    fn test_suggest_fix_unknown_returns_none() {
        let s = suggest_fix("E9999", "");
        assert!(s.is_none());

        let s = suggest_fix("UNKNOWN", "");
        assert!(s.is_none());
    }
}
