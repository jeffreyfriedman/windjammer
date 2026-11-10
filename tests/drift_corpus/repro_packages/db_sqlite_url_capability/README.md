# db_sqlite_url_capability

`wj test --interpret` passes. Under an explicit `[capabilities]` grant, the compiled run refuses (or, before that, fails to
build, see `runtime_feature_from_tests_only`) the `sqlite:<absolute path>` URL form for a database in the scratch directory
that the plain path opens: the check reads the scheme as part of a relative path. Owner: docs/drift/findings.md section L.
