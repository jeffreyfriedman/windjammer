//! Centralized type classification for the Windjammer compiler.
//!
//! This module is the **single source of truth** for classifying type names,
//! trait names, constructor names, and container types. ALL code that needs
//! to know about type categories MUST query this module instead of scattering
//! hardcoded `matches!("i8" | "i16" | ...)` across the codebase.
//!
//! For method-level classification, see `method_registry`.

// =============================================================================
// Primitive Type Classification
// =============================================================================

/// Integer types recognized by the Windjammer compiler.
pub fn is_integer_type(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
    )
}

/// Float types recognized by the Windjammer compiler.
pub fn is_float_type(name: &str) -> bool {
    matches!(name, "f32" | "f64")
}

/// All numeric types (integer + float).
pub fn is_numeric_type(name: &str) -> bool {
    is_integer_type(name) || is_float_type(name)
}

/// Primitive types that implement `Copy` (numeric + bool + char).
pub fn is_copy_primitive(name: &str) -> bool {
    is_numeric_type(name) || matches!(name, "bool" | "char")
}

/// Valid numeric literal suffixes in the lexer.
pub fn is_numeric_suffix(name: &str) -> bool {
    matches!(
        name,
        "u64" | "i64" | "u32" | "i32" | "u16" | "i16" | "u8" | "i8" | "usize" | "isize"
    )
}

/// Types that never need `use` imports (prelude, primitives, or synthesized).
pub fn is_prelude_or_primitive(name: &str) -> bool {
    is_copy_primitive(name) || matches!(name, "str" | "string" | "String" | "Self" | "self" | "()")
}

// =============================================================================
// Container / Stdlib Type Classification
// =============================================================================

/// Standard library generic containers — types that don't need `use` imports
/// and whose inner type arguments should be recursed into.
pub fn is_stdlib_container(name: &str) -> bool {
    matches!(
        name,
        "Vec"
            | "Option"
            | "Result"
            | "HashMap"
            | "HashSet"
            | "BTreeMap"
            | "BTreeSet"
            | "Box"
            | "Arc"
            | "Rc"
            | "RefCell"
            | "Cell"
            | "Mutex"
            | "RwLock"
            | "Weak"
            | "Pin"
            | "PhantomData"
            | "NonNull"
            | "VecDeque"
            | "BinaryHeap"
            | "LinkedList"
            | "SmallVec"
            | "Cow"
            | "Iter"
            | "Slice"
            | "Signal"
    )
}

/// Large collection types (high heap allocation cost, never Copy).
pub fn is_large_collection(name: &str) -> bool {
    matches!(
        name,
        "HashMap" | "BTreeMap" | "HashSet" | "BTreeSet" | "IndexMap"
    )
}

/// Medium-sized collection types.
pub fn is_medium_collection(name: &str) -> bool {
    matches!(name, "Vec" | "VecDeque" | "LinkedList")
}

/// Map/associative container types.
pub fn is_map_type(name: &str) -> bool {
    matches!(name, "HashMap" | "BTreeMap" | "IndexMap" | "Map")
}

/// Heap-owning types that are never Copy.
pub fn is_heap_container(name: &str) -> bool {
    matches!(name, "Vec" | "HashMap" | "String")
}

// =============================================================================
// Drop / Lifetime Classification
// =============================================================================

/// Types with important `Drop` semantics (not safe to defer dropping).
pub fn has_significant_drop(name: &str) -> bool {
    matches!(
        name,
        "Mutex"
            | "RwLock"
            | "File"
            | "TcpStream"
            | "UdpSocket"
            | "Channel"
            | "Receiver"
            | "Sender"
            | "JoinHandle"
            | "MutexGuard"
            | "RwLockReadGuard"
            | "RwLockWriteGuard"
    )
}

// =============================================================================
// Trait Classification
// =============================================================================

/// Operator traits that consume `self` (owned receiver).
pub fn is_consuming_operator_trait(name: &str) -> bool {
    let base = name.rsplit("::").next().unwrap_or(name);
    matches!(
        base,
        "Add"
            | "Sub"
            | "Mul"
            | "Div"
            | "Rem"
            | "Neg"
            | "Not"
            | "BitAnd"
            | "BitOr"
            | "BitXor"
            | "Shl"
            | "Shr"
    )
}

/// Conversion traits that consume `self` (owned receiver).
pub fn is_consuming_conversion_trait(name: &str) -> bool {
    let base = name.rsplit("::").next().unwrap_or(name);
    matches!(base, "Into" | "From" | "TryInto" | "TryFrom")
}

/// All traits where `self` should be owned (operators + conversions).
pub fn is_owned_self_trait(name: &str) -> bool {
    is_consuming_operator_trait(name) || is_consuming_conversion_trait(name)
}

/// Derive-style traits that use `&self` (borrowed receiver).
pub fn is_ref_receiver_trait(name: &str) -> bool {
    let base = name.rsplit("::").next().unwrap_or(name);
    matches!(
        base,
        "Display"
            | "Debug"
            | "Hash"
            | "PartialEq"
            | "Eq"
            | "PartialOrd"
            | "Ord"
            | "Clone"
            | "Copy"
            | "Default"
            | "Iterator"
            | "IntoIterator"
            | "AsRef"
            | "Deref"
    )
}

// =============================================================================
// Constructor / Factory Names
// =============================================================================

/// Common constructor / factory method names (no `self` receiver).
pub fn is_constructor_name(name: &str) -> bool {
    matches!(
        name,
        "new"
            | "default"
            | "from"
            | "from_str"
            | "from_bytes"
            | "with_capacity"
            | "empty"
            | "zero"
            | "one"
    )
}

// =============================================================================
// Method Classification (ownership-producing methods)
// =============================================================================

/// Methods that produce an owned value regardless of receiver ownership.
pub fn is_ownership_producing_method(name: &str) -> bool {
    matches!(name, "clone" | "to_owned" | "to_string" | "into_iter")
}

/// Methods that operate on float receivers and whose arguments should
/// match the receiver's float type.
pub fn is_float_receiver_method(name: &str) -> bool {
    matches!(
        name,
        "clamp"
            | "max"
            | "min"
            | "abs"
            | "copysign"
            | "recip"
            | "to_degrees"
            | "to_radians"
            | "signum"
            | "powf"
            | "powi"
            | "sqrt"
            | "cbrt"
            | "log"
            | "log2"
            | "log10"
            | "exp"
            | "exp2"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "sinh"
            | "cosh"
            | "tanh"
            | "ceil"
            | "floor"
            | "round"
            | "fract"
            | "trunc"
            | "hypot"
            | "mul_add"
            | "ln"
            | "fma"
    )
}

/// Storage methods that move a value into a collection (push, insert, etc.).
/// Used to determine if a parameter flows into a container field.
pub fn is_storage_method(name: &str) -> bool {
    matches!(
        name,
        "push" | "insert" | "extend" | "append" | "add" | "push_back" | "push_front"
    )
}

// =============================================================================
// Rust-to-Windjammer Type Mapping
// =============================================================================

/// Map a Rust type name to a Windjammer type name (for diagnostics).
pub fn rust_type_to_windjammer(rust_type: &str) -> &str {
    match rust_type {
        "i32" | "i64" | "isize" => "int",
        "u32" | "u64" | "usize" => "uint",
        "f32" | "f64" => "float",
        "&str" | "String" => "string",
        "bool" => "bool",
        "()" => "void",
        other => other,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_types() {
        assert!(is_integer_type("i32"));
        assert!(is_integer_type("usize"));
        assert!(!is_integer_type("f32"));
        assert!(!is_integer_type("bool"));
        assert!(!is_integer_type("String"));
    }

    #[test]
    fn test_copy_primitives() {
        assert!(is_copy_primitive("i32"));
        assert!(is_copy_primitive("f64"));
        assert!(is_copy_primitive("bool"));
        assert!(is_copy_primitive("char"));
        assert!(!is_copy_primitive("String"));
        assert!(!is_copy_primitive("Vec"));
    }

    #[test]
    fn test_stdlib_containers() {
        assert!(is_stdlib_container("Vec"));
        assert!(is_stdlib_container("HashMap"));
        assert!(is_stdlib_container("Option"));
        assert!(!is_stdlib_container("MyStruct"));
    }

    #[test]
    fn test_trait_classification() {
        assert!(is_consuming_operator_trait("Add"));
        assert!(is_consuming_operator_trait("std::ops::Sub"));
        assert!(!is_consuming_operator_trait("Display"));

        assert!(is_ref_receiver_trait("Debug"));
        assert!(is_ref_receiver_trait("Clone"));
        assert!(!is_ref_receiver_trait("Add"));

        assert!(is_owned_self_trait("Into"));
        assert!(is_owned_self_trait("Add"));
        assert!(!is_owned_self_trait("Display"));
    }

    #[test]
    fn test_constructor_names() {
        assert!(is_constructor_name("new"));
        assert!(is_constructor_name("default"));
        assert!(is_constructor_name("from"));
        assert!(!is_constructor_name("update"));
    }

    #[test]
    fn test_prelude_or_primitive() {
        assert!(is_prelude_or_primitive("i32"));
        assert!(is_prelude_or_primitive("String"));
        assert!(is_prelude_or_primitive("Self"));
        assert!(!is_prelude_or_primitive("MyType"));
    }

    #[test]
    fn test_float_methods() {
        assert!(is_float_receiver_method("clamp"));
        assert!(is_float_receiver_method("sin"));
        assert!(!is_float_receiver_method("push"));
    }

    #[test]
    fn test_significant_drop() {
        assert!(has_significant_drop("Mutex"));
        assert!(has_significant_drop("File"));
        assert!(!has_significant_drop("Vec"));
    }
}
