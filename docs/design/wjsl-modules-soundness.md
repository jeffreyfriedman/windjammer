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
| 8 | Renaming is binding-aware: parameters, `let`/`var`/`const` locals and `for` variables shadow module-level and imported names from the end of their declaration to the end of their block (initializers still see the outer name). Field accesses (`.x`), declared field names (`name:`) and attributes (`@location`) keep their spelling; a typed module-level `const NAME: T` is renamed. | `rewrite` (scope stack) | the `shadow_*` tests and `struct_fields_and_field_accesses_keep_their_spelling_next_to_an_imported_name` |

Known S1 limits, all deferred to S1b by the design: submodule folders, dependency packages, `pub use`, `super::`/`self::`.

## Mutation sweep

Each row is a temporary change to `src/wjsl/modules.rs` (an environment-switched mutant in a scratch build, never
committed); the named tests in `tests/wjsl_modules_test.rs` fail with it and pass without it.

| Mutant | Killed by |
|---|---|
| explicit import ignores `pub` | `s1_private_item_import_is_rejected_at_the_use_site_with_the_pub_fix` |
| qualified use ignores `pub` | `qualified_use_of_a_private_item_is_rejected_with_the_call_position` |
| cycle detection off | `a_cycle_that_returns_to_the_entry_file_is_reported_with_the_entry_name`, `s1_missing_module_item_cycle_reserved_name_and_library_binding_are_rejected` |
| library restriction off | `library_entry_points_globals_and_stray_pub_are_rejected` |
| `__` rejection off | `s1_missing_module_item_cycle_reserved_name_and_library_binding_are_rejected` |
| glob imports ignored | `glob_import_brings_every_pub_item_and_ranks_below_local_names`, `two_globs_bringing_one_name_are_an_error_naming_both_modules` |
| two-glob conflict ignored | `two_globs_bringing_one_name_are_an_error_naming_both_modules` |
| glob outranks local names | `glob_import_brings_every_pub_item_and_ranks_below_local_names` |
| `{a, b}` lists keep only the first name | `multi_line_import_lists_are_parsed_with_per_name_positions` and the golden |
| local-name collision ignored / import collision ignored | `a_local_item_or_a_second_import_with_an_imported_name_is_a_collision_error` |
| module order follows use-line order | `module_order_does_not_depend_on_the_order_of_use_lines` |
| text include not resolved (entry / library) | `s0_build_resolves_text_include`, `text_includes_and_module_imports_work_together_in_entry_and_library_files` |
| check runs on the pruned module | `unreached_library_code_is_still_checked_before_it_is_pruned` |
| origin comment dropped | `s1_same_folder_use_emits_one_module_matching_golden` |
| module alias not recorded | `s1_qualified_use_equals_direct_import`, `s1_same_name_in_two_modules_gets_distinct_names` |
| reachability not transitive / DCE off | `s1_same_folder_use_emits_one_module_matching_golden` |
| mangling off | the golden and 14 more |
| parameters / `for` variables / block end / declaration timing / `let` not tracked | the matching `shadow_*` tests |
| typed module-level `const` not renamed | `glob_import_...`, `multi_line_import_lists_...`, `shadow_in_a_nested_block_ends_with_the_block` |
| `.field` accesses renamed | `struct_fields_and_field_accesses_keep_their_spelling_next_to_an_imported_name` |
