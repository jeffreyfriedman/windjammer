# runtime_feature_from_tests_only

`wj test --interpret` passes. `wj test` (compiled) fails with E0432 (`windjammer_runtime::db` configured out): the runtime
feature groups are decided from the library's generated Rust only, so a test file that is the sole user of `std::db` gets
the import without the `db` feature. Owner: docs/drift/findings.md section L.
