//! Codegen Helper Functions
//!
//! Pure utility functions for code generation tasks like location extraction
//! and where-clause formatting. These functions have no state dependencies
//! and can be used independently.

use crate::parser::{Expression, Item, Statement};
use crate::source_map::Location;

/// Extract location from an Expression
///
/// Returns the location information attached to the expression, if any.
/// Used for error reporting and source mapping.
pub fn get_expression_location(expr: &Expression) -> Option<Location> {
    expr.location().clone()
}

/// Extract location from a Statement
///
/// Returns the location information attached to the statement, if any.
/// Used for error reporting and source mapping.
pub fn get_statement_location(stmt: &Statement) -> Option<Location> {
    stmt.location().clone()
}

/// Extract location from an Item
///
/// Returns the location information attached to the item, if any.
/// Used for error reporting and source mapping.
pub fn get_item_location(item: &Item) -> Option<Location> {
    item.location().clone()
}

/// Format where clause for Rust output
///
/// Converts a list of type parameter constraints into a properly formatted
/// where clause for generated Rust code.
///
/// # Examples
/// - Empty: `""` (no where clause)
/// - Single: `[("T", ["Display"])]` → `"\nwhere\n    T: Display"`
/// - Multiple bounds: `[("T", ["Display", "Clone"])]` → `"\nwhere\n    T: Display + Clone"`
/// - Multiple params: `[("T", ["Display"]), ("U", ["Debug"])]` → `"\nwhere\n    T: Display,\n    U: Debug"`
pub fn format_where_clause(where_clause: &[(String, Vec<String>)]) -> String {
    if where_clause.is_empty() {
        return String::new();
    }

    let clauses: Vec<String> = where_clause
        .iter()
        .map(|(type_param, bounds)| format!("    {}: {}", type_param, bounds.join(" + ")))
        .collect();

    format!("\nwhere\n{}", clauses.join(",\n"))
}
