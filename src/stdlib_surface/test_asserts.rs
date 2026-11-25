//! The `std::test` functions the compiler lowers to `windjammer_runtime::test::<name>(..)`, and the two facts it needs
//! about them. One table: codegen, float inference and the prelude read it instead of matching names themselves.

/// Every canonical assertion function that is a runtime call (`assert`, `assert_eq`, `assert_ne` are intrinsics).
pub const RUNTIME_FUNCTIONS: &[&str] = &[
    "assert_gt",
    "assert_lt",
    "assert_ge",
    "assert_le",
    "assert_approx_eq",
    "assert_in_range",
    "assert_some",
    "assert_none",
    "assert_ok",
    "assert_err",
    "assert_contains",
    "assert_starts_with",
    "assert_ends_with",
    "assert_empty",
    "assert_not_empty",
    "assert_length",
];

/// Functions that only look at their first argument, so the call borrows it and the caller keeps the value.
pub const BORROWS_FIRST_ARGUMENT: &[&str] =
    &["assert_contains", "assert_starts_with", "assert_ends_with", "assert_empty", "assert_not_empty", "assert_length"];

/// Functions whose first `n` arguments are one type `T` (`assert_approx_eq(x, 1.5, 0.001)` with `x: f32` makes the
/// literals `f32`): (name, operand count).
pub const SAME_TYPE_OPERANDS: &[(&str, usize)] =
    &[("assert_gt", 2), ("assert_lt", 2), ("assert_ge", 2), ("assert_le", 2), ("assert_approx_eq", 3), ("assert_in_range", 3)];

pub fn is_runtime_function(name: &str) -> bool {
    RUNTIME_FUNCTIONS.contains(&name)
}

pub fn borrows_first_argument(name: &str) -> bool {
    BORROWS_FIRST_ARGUMENT.contains(&name)
}

pub fn same_type_operands(name: &str) -> Option<usize> {
    SAME_TYPE_OPERANDS.iter().find(|(n, _)| *n == name).map(|(_, count)| *count)
}
