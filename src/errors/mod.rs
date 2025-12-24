/// Error Reporting Module
///
/// Provides high-quality, Rust-style error messages for Windjammer code.
pub mod mutability;

pub use mutability::{MutabilityChecker, MutabilityError, MutabilityErrorType};
