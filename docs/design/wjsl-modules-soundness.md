---
status: current
last_verified: 46e3e530143f
---
# WJSL modules: resolver invariants

The module resolver (`src/wjsl/modules.rs`, design [wjsl-modules.md](wjsl-modules.md)) is sound when the emitted WGSL
means what the source files mean and nothing else. Each invariant below names where it lives and the test that fails
when it is removed (all in `tests/wjsl_modules_test.rs`).

| # | Invariant | Where | Pinned by |
|---|---|---|---|
| 1 | An emitted name is a pure function of `(module stem, item name)`: `stem__name` for library items, the plain name for entry-file items. Adding or removing unrelated files renames nothing. | `qualified_name`, `resolve_imports` | `s1_names_are_stable_when_unrelated_files_are_added_or_removed`, `s1_same_name_in_two_modules_gets_distinct_names` |
| 2 | Mangled names cannot collide with user names: `__` is rejected in every identifier (WJ0201), so no user name contains the separator. | `check_reserved` | `s1_missing_module_item_cycle_reserved_name_and_library_binding_are_rejected` |
| 3 | Only reached items are emitted: reachability starts at every entry-file item and follows identifiers and struct types through library bodies, initializers and signatures. The checker still sees every library item before pruning. | `transpile` (dead-code elimination) | `s1_same_folder_use_emits_one_module_matching_golden` (no `unused_helper`, transitive `pow5` kept) |
| 4 | Output depends only on source text and module stems: modules are ordered by depth-first post-order with imports sorted by stem, items by source order; no path, directory listing or hash order is read. | `visit`, `Graph::order` | `s1_output_is_location_agnostic` |
| 5 | A library module holds only `fn`, `struct`, `const`; bindings, `var<..>` globals and entry points are WJ0200. | `load_module` | the rejection test above |
| 6 | Visibility: an import or qualified use of a non-`pub` item is WJ0198 at the use site. | `resolve_imports`, `rewrite` | `s1_private_item_import_is_rejected_at_the_use_site_with_the_pub_fix` |
| 7 | Import cycles are WJ0199 with the chain; a module is loaded once however many files import it. | `visit` | the rejection test above |
| 8 | Renaming skips field accesses (`.x`), declared names (`name:`) and attributes (`@location`), so struct fields, parameters and swizzles keep their spelling. Limit: a local variable or parameter named like a module-level item is rewritten at its uses; give them distinct names. | `rewrite` | `s1_same_folder_use_emits_one_module_matching_golden` |

Known S1 limits, all deferred to S1b by the design: submodule folders, dependency packages, `pub use`, `super::`/`self::`,
multi-line `{a, b}` import lists.
