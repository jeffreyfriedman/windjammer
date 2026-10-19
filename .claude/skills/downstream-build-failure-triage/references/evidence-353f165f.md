git show HEAD:src/stdlib_surface/enumerable.rs has no `is_surface_name`, so the burn-down in 99473791 looks like it removed it while `check/semantic/std_methods.rs` still calls it.
src/check/semantic/std_methods.rs:104:        || crate::stdlib_surface::enumerable::is_surface_name(name)
I fixed the break in `next/v0.50.0` and added a merge-gate step so the same thing can't slip through again.