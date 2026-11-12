//! The stdlib's effect declarations and taint sources/sinks, and the typed surface of the Rust core types. They are
//! functions of the stdlib only, yet each `IrPipeline` used to rebuild them (once per checked file). A compilation
//! session computes them once (`StdlibTables::compute`) and hands the same tables to all its pipelines
//! (`IrPipeline::with_stdlib_tables`).

use super::effects::{stdlib_effect_declarations, EffectConstraint};
use super::taint::{stdlib_sinks, stdlib_taint_sources, TaintConstraint};
pub use crate::typing::core_surface::{surface_type_name, CoreSurface, ITERATOR};

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
            core_surface: CoreSurface::parse(),
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
