// AST Types - Windjammer Abstract Syntax Tree Definitions
//
// This module contains all AST (Abstract Syntax Tree) type definitions for Windjammer.
// These types represent the parsed structure of Windjammer source code.

use crate::source_map::Location;

// ============================================================================
// SOURCE LOCATION
// ============================================================================

/// Optional source location for error reporting
/// None means location information is not available
pub type SourceLocation = Option<Location>;

// ============================================================================
// TYPE SYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Int32,
    Uint,
    Float,
    Bool,
    String,
    Custom(String),
    Generic(String),                  // Type parameter: T, U, V
    Parameterized(String, Vec<Type>), // Generic type: Vec<T>, HashMap<K, V>
    Associated(String, String),       // Associated type: Self::Item, T::Output (base, assoc_name)
    TraitObject(String),              // Trait object: dyn Trait
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Vec(Box<Type>),          // Dynamic array: Vec<T>
    Array(Box<Type>, usize), // Fixed-size array: [T; N]
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    Tuple(Vec<Type>), // Tuple type: (T1, T2, T3)
    Infer,            // Type inference placeholder: _
    FunctionPointer {
        params: Vec<Type>,
        return_type: Option<Box<Type>>,
    }, // Function pointer: fn(int, string) -> bool
}

// Type parameter with optional trait bounds
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>, // Trait bounds: ["Display", "Clone", "Send"]
}

// Associated type declaration in traits or implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedType {
    pub name: String,                // e.g., "Item", "Output"
    pub concrete_type: Option<Type>, // None in trait declaration, Some(Type) in impl
}
