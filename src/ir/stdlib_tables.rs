//! The stdlib's effect declarations and taint sources/sinks, and the typed surface of the Rust core types. They are
//! functions of the stdlib only, yet each `IrPipeline` used to rebuild them (once per checked file). A compilation
//! session computes them once (`StdlibTables::compute`) and hands the same tables to all its pipelines
//! (`IrPipeline::with_stdlib_tables`).

use super::effects::{stdlib_effect_declarations, EffectConstraint};
use super::taint::{stdlib_sinks, stdlib_taint_sources, TaintConstraint};
pub use crate::typing::core_surface::{surface_type_name, CoreSurface, ITERATOR};

/// `std/rust_core.wji` parsed once per process (kept with the stdlib registry, like the other derived tables): the
/// numeric solver's session copy, the portable backends and the Rust backend read the same parse.
pub fn shared_core_surface() -> &'static CoreSurface {
    struct Core(CoreSurface);
    &crate::analyzer::SignatureRegistry::stdlib().derived().get_or_build(|| Core(CoreSurface::parse())).0
}

/// The lowering table of [`shared_core_surface`] (the `@js` / `@go` rules the portable emitters interpret).
pub fn shared_lowering() -> &'static crate::stdlib_surface::lowering::LoweringTable {
    struct Lowering(crate::stdlib_surface::lowering::LoweringTable);
    let core = shared_core_surface(); // before the closure: the derived tables are built under a lock
    &crate::analyzer::SignatureRegistry::stdlib().derived().get_or_build(|| Lowering(core.lowering())).0
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StdlibTables {
    pub effect_declarations: Vec<EffectConstraint>,
    pub taint_sources: Vec<TaintConstraint>,
    pub taint_sinks: Vec<TaintConstraint>,
    /// `std/rust_core.wji` (option C stage C1): core-type method signatures for the numeric solver.
    pub core_surface: CoreSurface,
}

impl StdlibTables {
    /// The tables, from the shared store when it has this build's entry, else computed (and stored). They are
    /// functions of the build and the baked-in trees, so a stored copy is the computed one; any miss computes.
    pub fn load_or_compute() -> Self {
        let shared = crate::analyzer::shared_entry("stdlib-tables");
        if let Some((store, key)) = &shared {
            if let Some(t) = store.get(key).and_then(|text| serde_json::from_str::<Self>(&text).ok()) {
                return t;
            }
        }
        let tables = Self::compute();
        if let (Some((store, key)), Ok(text)) = (&shared, serde_json::to_string(&tables)) {
            store.put(key, &text);
        }
        tables
    }

    pub fn compute() -> Self {
        Self {
            effect_declarations: stdlib_effect_declarations(),
            taint_sources: stdlib_taint_sources(),
            taint_sinks: stdlib_sinks(),
            core_surface: shared_core_surface().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StdlibTables;

    /// The stored tables are the computed tables: the text survives a round trip unchanged.
    #[test]
    fn stored_tables_round_trip() {
        let tables = StdlibTables::compute();
        let text = serde_json::to_string(&tables).unwrap();
        let back: StdlibTables = serde_json::from_str(&text).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), text);
        assert_eq!(format!("{:?}", back.core_surface), format!("{:?}", tables.core_surface));
        assert!(!tables.taint_sinks.is_empty() && !tables.effect_declarations.is_empty());
    }
}
