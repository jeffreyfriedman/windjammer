# Rust target: an unannotated float literal does not take the `f32` of its context

Since the default for an unconstrained float literal became `float` (`f64`, SPEC-NUM-007), a literal that is
constrained only by a declared `f32` somewhere else keeps `f64` and rustc rejects the program (`E0308: expected f32,
found f64`). Where the constraint is a plain local, parameter or struct field the solver handles it; these shapes are
still missed:

| Shape | Repro (test) |
|-------|--------------|
| payload of an enum variant: `Value::Float(3.14)` with `Float(f32)` | `enum_struct_destructuring_test::test_enum_mixed_variants` |
| tuple field behind a `type` alias: `type Quat = (f32, f32, f32, f32)`; `rotation: (0.0, 0.0, 0.0, 1.0)` | `float_inference_field_initializer_test::test_struct_tuple_field_with_type_alias` |
| argument of a function defined in another module (`Vec3::new(1.0, 2.0, 3.0)`) | `type_inference_vec3_f32_test::test_vec3_cross_module_inference` |
| tuple element pushed into a `Vec<(string, f32)>` | `windjammer_fixtures_integration::regression_vec_tuple_string_push` |

Why: the Rust backend still decides the suffix per literal in `codegen/rust` from local type queries
(`int_type_from_assignment_target` and the float equivalents) instead of reading the unified numeric solver's answer
(`docs/spec/inference.md`, D-NUM-DEFAULT, convergence stage C4). Each shape above needs the expected type pushed
down through the enum payload, alias, cross-module signature or tuple element; with the solver as the only width owner
they disappear together.

These tests are listed in `.config/known-test-failures.txt` and must be removed from it when they pass.
