# Compiled tests: `let _empty = Vec::new()` / `HashMap::new()` that is never used fails with `E0282`

```windjammer
@test
fn test_empty_is_empty() {
    let _empty = Vec::new()
    assert(true, "unused collection")
}
```

The generated Rust is `let _empty = Vec::new();` and rustc reports `type annotations needed for Vec<_>`. This is the
shape of two ecosystem tests (`wj-multipart`, `wj-template`). The element type cannot be inferred in Windjammer either, so
the compiler should either infer a default (`Vec<()>`) for an unused empty collection or report a Windjammer-level error.

Status: fixed. An empty-collection local that nothing reads gets unit element types (`let _empty: Vec<()> = Vec::new()`, `HashMap<(), ()>`); a local that is used keeps its inferred type. Gates: `ecosystem_findings2::unused_empty_collection_local_gets_a_default_element_type`, `used_empty_collection_keeps_its_inferred_element_type`.
