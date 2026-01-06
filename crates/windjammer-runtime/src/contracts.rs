//! Design-by-contract utilities
//!
//! Provides runtime support for preconditions, postconditions, and invariants.

use std::fmt;

/// Contract violation error
#[derive(Debug, Clone)]
pub struct ContractViolation {
    pub kind: ContractKind,
    pub condition: String,
    pub message: Option<String>,
}

impl ContractViolation {
    pub fn new(kind: ContractKind, condition: String) -> Self {
        Self {
            kind,
            condition,
            message: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} violation: {}", self.kind, self.condition)?;
        if let Some(msg) = &self.message {
            write!(f, " ({})", msg)?;
        }
        Ok(())
    }
}

impl std::error::Error for ContractViolation {}

/// Contract kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractKind {
    Precondition,
    Postcondition,
    Invariant,
}

impl fmt::Display for ContractKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractKind::Precondition => write!(f, "Precondition"),
            ContractKind::Postcondition => write!(f, "Postcondition"),
            ContractKind::Invariant => write!(f, "Invariant"),
        }
    }
}

/// Check a precondition (requires)
///
/// # Example
/// ```
/// use windjammer_runtime::contracts::requires;
///
/// fn divide(a: i32, b: i32) -> i32 {
///     requires(b != 0, "divisor must be non-zero");
///     a / b
/// }
/// ```
pub fn requires(condition: bool, description: &str) {
    if !condition {
        panic!(
            "{}",
            ContractViolation::new(ContractKind::Precondition, description.to_string())
        );
    }
}

/// Check a postcondition (ensures)
///
/// # Example
/// ```
/// use windjammer_runtime::contracts::ensures;
///
/// fn abs(x: i32) -> i32 {
///     let result = if x < 0 { -x } else { x };
///     ensures(result >= 0, "result must be non-negative");
///     result
/// }
/// ```
pub fn ensures(condition: bool, description: &str) {
    if !condition {
        panic!(
            "{}",
            ContractViolation::new(ContractKind::Postcondition, description.to_string())
        );
    }
}

/// Check an invariant
///
/// # Example
/// ```
/// use windjammer_runtime::contracts::invariant;
///
/// struct Counter {
///     count: i32,
/// }
///
/// impl Counter {
///     fn increment(&mut self) {
///         self.count += 1;
///         invariant(self.count >= 0, "count must be non-negative");
///     }
/// }
/// ```
pub fn invariant(condition: bool, description: &str) {
    if !condition {
        panic!(
            "{}",
            ContractViolation::new(ContractKind::Invariant, description.to_string())
        );
    }
}

/// Helper for capturing "old" values in postconditions
///
/// # Example
/// ```
/// use windjammer_runtime::contracts::old;
///
/// fn increment(x: &mut i32) {
///     let old_x = old(*x);
///     *x += 1;
///     assert_eq!(*x, old_x + 1);
/// }
/// ```
#[inline(always)]
pub fn old<T: Clone>(value: T) -> T {
    value.clone()
}

/// Contract builder for complex contracts
pub struct Contract {
    preconditions: Vec<(bool, String)>,
    postconditions: Vec<(bool, String)>,
    invariants: Vec<(bool, String)>,
}

impl Contract {
    pub fn new() -> Self {
        Self {
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
        }
    }

    pub fn requires(mut self, condition: bool, description: &str) -> Self {
        self.preconditions
            .push((condition, description.to_string()));
        self
    }

    pub fn ensures(mut self, condition: bool, description: &str) -> Self {
        self.postconditions
            .push((condition, description.to_string()));
        self
    }

    pub fn invariant(mut self, condition: bool, description: &str) -> Self {
        self.invariants.push((condition, description.to_string()));
        self
    }

    pub fn check_preconditions(&self) {
        for (condition, desc) in &self.preconditions {
            if !condition {
                panic!(
                    "{}",
                    ContractViolation::new(ContractKind::Precondition, desc.clone())
                );
            }
        }
    }

    pub fn check_postconditions(&self) {
        for (condition, desc) in &self.postconditions {
            if !condition {
                panic!(
                    "{}",
                    ContractViolation::new(ContractKind::Postcondition, desc.clone())
                );
            }
        }
    }

    pub fn check_invariants(&self) {
        for (condition, desc) in &self.invariants {
            if !condition {
                panic!(
                    "{}",
                    ContractViolation::new(ContractKind::Invariant, desc.clone())
                );
            }
        }
    }
}

impl Default for Contract {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requires_success() {
        requires(true, "should not panic");
        // Test passes
    }

    #[test]
    #[should_panic(expected = "Precondition violation")]
    fn test_requires_failure() {
        requires(false, "should panic");
    }

    #[test]
    fn test_ensures_success() {
        ensures(true, "should not panic");
        // Test passes
    }

    #[test]
    #[should_panic(expected = "Postcondition violation")]
    fn test_ensures_failure() {
        ensures(false, "should panic");
    }

    #[test]
    fn test_invariant_success() {
        invariant(true, "should not panic");
        // Test passes
    }

    #[test]
    #[should_panic(expected = "Invariant violation")]
    fn test_invariant_failure() {
        invariant(false, "should panic");
    }

    #[test]
    fn test_old() {
        let x = 42;
        let old_x = old(x);
        assert_eq!(old_x, 42);
    }

    #[test]
    fn test_contract_builder() {
        let contract = Contract::new()
            .requires(true, "x > 0")
            .requires(true, "y > 0")
            .ensures(true, "result > 0");

        contract.check_preconditions();
        contract.check_postconditions();
    }

    #[test]
    #[should_panic(expected = "Precondition violation")]
    fn test_contract_precondition_failure() {
        let contract = Contract::new().requires(false, "x > 0");
        contract.check_preconditions();
    }

    #[test]
    #[should_panic(expected = "Postcondition violation")]
    fn test_contract_postcondition_failure() {
        let contract = Contract::new().ensures(false, "result > 0");
        contract.check_postconditions();
    }

    #[test]
    #[should_panic(expected = "Invariant violation")]
    fn test_contract_invariant_failure() {
        let contract = Contract::new().invariant(false, "count >= 0");
        contract.check_invariants();
    }

    #[test]
    fn test_contract_violation_display() {
        let violation =
            ContractViolation::new(ContractKind::Precondition, "x > 0".to_string());
        assert_eq!(violation.to_string(), "Precondition violation: x > 0");
    }

    #[test]
    fn test_contract_violation_with_message() {
        let violation = ContractViolation::new(ContractKind::Precondition, "x > 0".to_string())
            .with_message("x must be positive".to_string());
        assert_eq!(
            violation.to_string(),
            "Precondition violation: x > 0 (x must be positive)"
        );
    }
}

