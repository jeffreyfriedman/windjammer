//! String pool construction and static item generation.

use crate::parser::{Expression, Item, Literal, Type};
use std::collections::HashMap;

/// String pool entry
#[derive(Debug, Clone)]
pub(super) struct StringPoolEntry {
    /// The string literal value
    pub(super) value: String,
    /// Number of occurrences
    pub(super) count: usize,
    /// Generated pool variable name
    pub(super) pool_name: String,
}

/// Build string pool from frequency analysis
pub(super) fn build_string_pool(frequency: HashMap<String, usize>) -> Vec<StringPoolEntry> {
    let mut pool = Vec::new();
    let mut index = 0;

    for (value, count) in frequency {
        // Only intern strings that appear 2+ times
        if count >= 2 {
            pool.push(StringPoolEntry {
                value: value.clone(),
                count,
                pool_name: format!("__STRING_POOL_{}", index),
            });
            index += 1;
        }
    }

    // Sort by count (most frequent first) for better cache locality
    pool.sort_by_key(|b| std::cmp::Reverse(b.count));

    pool
}

/// Create a map from string value to pool name for quick lookup
pub(super) fn create_pool_map(pool: &[StringPoolEntry]) -> HashMap<String, String> {
    pool.iter()
        .map(|entry| (entry.value.clone(), entry.pool_name.clone()))
        .collect()
}

/// Create static declarations for string pool
pub(super) fn create_pool_statics<'ast>(
    pool: &[StringPoolEntry],
    optimizer: &crate::optimizer::Optimizer,
) -> Vec<Item<'ast>> {
    pool
        .iter()
        .map(|entry| Item::Static {
            name: entry.pool_name.clone(),
            mutable: false,
            type_: Type::Reference(Box::new(Type::Custom("str".to_string()))),
            value: optimizer.alloc_expr(Expression::Literal {
                value: Literal::String(entry.value.clone()),
                location: None,
            }),
            location: None,
        })
        .collect()
}
