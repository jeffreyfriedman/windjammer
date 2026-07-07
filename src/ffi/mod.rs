//! C FFI binding generation from analyzed module metadata (WJ-IMPL-02).
//!
//! Converts Windjammer `.wj.meta` / `ModuleMetadata` into an IDL representation
//! and generates C header declarations for SDK consumers.

pub mod c_bindings;
pub mod idl;

pub use c_bindings::generate_c_header;
pub use idl::{
    IdlField, IdlFunction, IdlModule, IdlParam, IdlType, IdlTypeKind,
};
