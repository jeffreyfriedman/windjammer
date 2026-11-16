//! A fn-typed parameter of a library function survives the crate metadata (Debug form): a closure passed to it in a
//! dependent crate (a test crate) takes its parameter types from it, so `|n| n >= 0` with `prop: fn(int) -> bool`
//! compares `n: i64` with `0_i64` (it was `0_i32` when the parameter type was dropped).

use windjammer::metadata::ModuleMetadata;
use windjammer::parser::Type;

#[test]
fn function_pointer_types_round_trip_through_metadata() {
    for t in [
        Type::FunctionPointer { params: vec![Type::Int], return_type: Some(Box::new(Type::Bool)) },
        Type::FunctionPointer { params: vec![Type::Int, Type::Custom("Config".into())], return_type: None },
        Type::FunctionPointer { params: vec![], return_type: Some(Box::new(Type::Vec(Box::new(Type::Int)))) },
        Type::Vec(Box::new(Type::FunctionPointer { params: vec![Type::String], return_type: Some(Box::new(Type::Int)) })),
    ] {
        let text = format!("{t:?}");
        assert_eq!(ModuleMetadata::deserialize_type(&text), Some(t.clone()), "{text}");
    }
}
