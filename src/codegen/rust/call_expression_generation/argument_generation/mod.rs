//! Argument strings for calls: `Call(FieldAccess)` method-style args and plain function args.

mod field_access_method_args;
mod regular_call_arguments;

pub(in crate::codegen::rust) use field_access_method_args::{
    field_access_method_args_fallback, field_access_method_args_with_signature,
};
pub(in crate::codegen::rust) use regular_call_arguments::collect_regular_function_arguments;
