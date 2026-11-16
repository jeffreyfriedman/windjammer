# test_returning_err

An `@test` function returning `Result` that returns `Err`. Compiled (`wj test`): `test_err` fails (Rust prints
`Error: "not one: 2"`), `test_ok` passes. Interpreted (`wj test --interpret`) reported both as ok until the runner
checked the returned value; it now fails `test_err` with the `Err` value. Owner: interpreter (fixed on
wt/interp-wc2); gate `tests/interpreter_compiled_agreement_test.rs`
`a_test_returning_err_fails_interpreted_like_the_compiled_harness`.
