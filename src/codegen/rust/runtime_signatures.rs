//! The runtime's own signatures of standard-library functions.

use crate::analyzer::{FunctionSignature, SignatureRegistry};

/// The scanned Rust API of a standard-library function (`http::get`), which is authoritative over the
/// `std/<mod>.wj` stub's declaration (a stub body is empty, so its formals say nothing about the runtime).
pub fn runtime_signature(qualified: &str) -> Option<&'static FunctionSignature> {
    let stdlib = SignatureRegistry::stdlib();
    stdlib.get_signature(qualified).or_else(|| stdlib.get_fallback_signature(qualified))
}
