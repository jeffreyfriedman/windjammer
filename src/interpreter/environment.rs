//! Scope-based environment for variable bindings.
//!
//! Supports nested scopes (function calls, blocks, loops) with
//! variable shadowing. Uses a scope stack — entering a function
//! or block pushes a new scope, leaving pops it.

use super::value::Value;
use std::collections::HashMap;

/// A single scope level (function, block, loop, etc.)
#[derive(Debug, Clone)]
struct Scope {
    bindings: HashMap<String, Value>,
}

/// The environment: a stack of scopes
#[derive(Debug, Clone)]
pub struct Environment {
    scopes: Vec<Scope>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope {
                bindings: HashMap::new(),
            }],
        }
    }

    /// Push a new scope (entering a block/function)
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope {
            bindings: HashMap::new(),
        });
    }

    /// Pop the top scope (leaving a block/function)
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a new variable in the current (top) scope
    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), value);
        }
    }

    /// Look up a variable by name (search from innermost to outermost scope)
    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.bindings.get(name) {
                return Some(val);
            }
        }
        None
    }

    /// Assign to an existing variable (search from innermost out)
    pub fn set(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.bindings.contains_key(name) {
                scope.bindings.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }

    /// Get a mutable reference to a struct's field (for field mutation)
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(val) = scope.bindings.get_mut(name) {
                return Some(val);
            }
        }
        None
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
