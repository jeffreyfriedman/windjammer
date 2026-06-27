//! Effect system implementation (WJ-SEC-01).
//!
//! Effects track what side-effects a function may perform: filesystem I/O,
//! network access, process spawning, etc. The solver propagates effects up
//! the call graph automatically. The application manifest declares allowed
//! effects; violations are compile errors.

use crate::ir::safety_type::{Effect, EffectSet};
use std::collections::{HashMap, HashSet};

/// An effect constraint for the solver.
#[derive(Debug, Clone)]
pub enum EffectConstraint {
    /// A function directly performs this effect (leaf constraint from stdlib).
    Performs { function: String, effect: Effect },
    /// A function's effects include all effects of its callees.
    CallsInto { caller: String, callee: String },
    /// A function is declared pure (no effects allowed).
    IsPure { function: String },
}

/// The effect solver — propagates effects up the call graph.
#[derive(Default)]
pub struct EffectSolver {
    /// Direct effects declared for each function (from stdlib annotations).
    direct_effects: HashMap<String, EffectSet>,
    /// Call graph edges: caller -> set of callees.
    call_graph: HashMap<String, HashSet<String>>,
    /// Functions declared pure.
    pure_functions: HashSet<String>,
}

impl EffectSolver {
    pub fn new() -> Self {
        Self {
            direct_effects: HashMap::new(),
            call_graph: HashMap::new(),
            pure_functions: HashSet::new(),
        }
    }

    /// Add a constraint to the solver.
    pub fn add_constraint(&mut self, constraint: EffectConstraint) {
        match constraint {
            EffectConstraint::Performs { function, effect } => {
                self.direct_effects
                    .entry(function)
                    .or_insert_with(EffectSet::pure)
                    .insert(effect);
            }
            EffectConstraint::CallsInto { caller, callee } => {
                self.call_graph.entry(caller).or_default().insert(callee);
            }
            EffectConstraint::IsPure { function } => {
                self.pure_functions.insert(function);
            }
        }
    }

    /// Solve: propagate effects up the call graph.
    /// Returns the full effect set for every function.
    pub fn solve(&self) -> EffectSolverResult {
        let mut resolved: HashMap<String, EffectSet> = HashMap::new();
        let mut errors: Vec<EffectError> = Vec::new();

        // Start with direct effects
        for (func, effects) in &self.direct_effects {
            resolved.insert(func.clone(), effects.clone());
        }

        // Propagate up the call graph (fixpoint iteration)
        let max_iterations = 100;
        let mut changed = true;
        let mut iteration = 0;

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for (caller, callees) in &self.call_graph {
                let mut caller_effects = resolved
                    .get(caller)
                    .cloned()
                    .unwrap_or_else(EffectSet::pure);

                for callee in callees {
                    if let Some(callee_effects) = resolved.get(callee) {
                        let merged = caller_effects.union(callee_effects);
                        if merged != caller_effects {
                            caller_effects = merged;
                            changed = true;
                        }
                    }
                }

                resolved.insert(caller.clone(), caller_effects);
            }
        }

        // Check pure function violations
        for func in &self.pure_functions {
            if let Some(effects) = resolved.get(func) {
                if !effects.is_pure() {
                    errors.push(EffectError {
                        kind: EffectErrorKind::PurityViolation,
                        function: func.clone(),
                        message: format!(
                            "function '{}' is declared pure but has effects: {:?}",
                            func,
                            effects.iter().collect::<Vec<_>>()
                        ),
                    });
                }
            }
        }

        EffectSolverResult { resolved, errors }
    }

    /// Check if a program's effects satisfy a manifest's allowed effects.
    pub fn check_manifest(
        &self,
        result: &EffectSolverResult,
        entry_point: &str,
        allowed: &EffectSet,
    ) -> Vec<EffectError> {
        let mut errors = Vec::new();

        if let Some(entry_effects) = result.resolved.get(entry_point) {
            if !entry_effects.is_subset_of(allowed) {
                let violations: Vec<&Effect> = entry_effects
                    .iter()
                    .filter(|e| !allowed.contains(e))
                    .collect();

                errors.push(EffectError {
                    kind: EffectErrorKind::ManifestViolation,
                    function: entry_point.to_string(),
                    message: format!("program requires effects not in manifest: {:?}", violations),
                });
            }
        }

        errors
    }
}

/// Result of effect solving.
#[derive(Debug)]
pub struct EffectSolverResult {
    /// Computed effects for each function.
    pub resolved: HashMap<String, EffectSet>,
    /// Errors (purity violations, etc.).
    pub errors: Vec<EffectError>,
}

/// An effect system error.
#[derive(Debug, Clone)]
pub struct EffectError {
    pub kind: EffectErrorKind,
    pub function: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectErrorKind {
    PurityViolation,
    ManifestViolation,
}

/// Standard library effect declarations.
/// These define which stdlib functions have which effects.
pub fn stdlib_effect_declarations() -> Vec<EffectConstraint> {
    vec![
        // Filesystem
        EffectConstraint::Performs {
            function: "std::fs::read".into(),
            effect: Effect::FsRead,
        },
        EffectConstraint::Performs {
            function: "std::fs::write".into(),
            effect: Effect::FsWrite,
        },
        EffectConstraint::Performs {
            function: "std::fs::read_to_string".into(),
            effect: Effect::FsRead,
        },
        EffectConstraint::Performs {
            function: "std::fs::create_dir".into(),
            effect: Effect::FsWrite,
        },
        // Network
        EffectConstraint::Performs {
            function: "std::http::get".into(),
            effect: Effect::NetEgress,
        },
        EffectConstraint::Performs {
            function: "std::http::post".into(),
            effect: Effect::NetEgress,
        },
        EffectConstraint::Performs {
            function: "std::http::listen".into(),
            effect: Effect::NetIngress,
        },
        // Process
        EffectConstraint::Performs {
            function: "std::process::spawn".into(),
            effect: Effect::ProcessSpawn,
        },
        EffectConstraint::Performs {
            function: "std::process::exec".into(),
            effect: Effect::ProcessSpawn,
        },
        // Environment
        EffectConstraint::Performs {
            function: "std::env::get".into(),
            effect: Effect::EnvRead,
        },
        EffectConstraint::Performs {
            function: "std::env::set".into(),
            effect: Effect::EnvWrite,
        },
        // Database
        EffectConstraint::Performs {
            function: "std::db::query".into(),
            effect: Effect::NetEgress,
        },
        EffectConstraint::Performs {
            function: "std::db::execute".into(),
            effect: Effect::NetEgress,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_effects() {
        let mut solver = EffectSolver::new();
        solver.add_constraint(EffectConstraint::Performs {
            function: "read_file".into(),
            effect: Effect::FsRead,
        });

        let result = solver.solve();
        let effects = result.resolved.get("read_file").unwrap();
        assert!(effects.contains(&Effect::FsRead));
        assert!(!effects.contains(&Effect::NetEgress));
    }

    #[test]
    fn test_effect_propagation() {
        let mut solver = EffectSolver::new();

        // read_file performs fs_read
        solver.add_constraint(EffectConstraint::Performs {
            function: "read_file".into(),
            effect: Effect::FsRead,
        });

        // process calls read_file
        solver.add_constraint(EffectConstraint::CallsInto {
            caller: "process".into(),
            callee: "read_file".into(),
        });

        // main calls process
        solver.add_constraint(EffectConstraint::CallsInto {
            caller: "main".into(),
            callee: "process".into(),
        });

        let result = solver.solve();

        // Effects should propagate: read_file -> process -> main
        assert!(result
            .resolved
            .get("process")
            .unwrap()
            .contains(&Effect::FsRead));
        assert!(result
            .resolved
            .get("main")
            .unwrap()
            .contains(&Effect::FsRead));
    }

    #[test]
    fn test_effect_union() {
        let mut solver = EffectSolver::new();

        solver.add_constraint(EffectConstraint::Performs {
            function: "fetch".into(),
            effect: Effect::NetEgress,
        });
        solver.add_constraint(EffectConstraint::Performs {
            function: "save".into(),
            effect: Effect::FsWrite,
        });

        // handler calls both
        solver.add_constraint(EffectConstraint::CallsInto {
            caller: "handler".into(),
            callee: "fetch".into(),
        });
        solver.add_constraint(EffectConstraint::CallsInto {
            caller: "handler".into(),
            callee: "save".into(),
        });

        let result = solver.solve();
        let handler_effects = result.resolved.get("handler").unwrap();
        assert!(handler_effects.contains(&Effect::NetEgress));
        assert!(handler_effects.contains(&Effect::FsWrite));
    }

    #[test]
    fn test_purity_violation() {
        let mut solver = EffectSolver::new();

        solver.add_constraint(EffectConstraint::Performs {
            function: "read_file".into(),
            effect: Effect::FsRead,
        });
        solver.add_constraint(EffectConstraint::CallsInto {
            caller: "compute".into(),
            callee: "read_file".into(),
        });
        solver.add_constraint(EffectConstraint::IsPure {
            function: "compute".into(),
        });

        let result = solver.solve();
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, EffectErrorKind::PurityViolation);
    }

    #[test]
    fn test_manifest_check_passes() {
        let mut solver = EffectSolver::new();
        solver.add_constraint(EffectConstraint::Performs {
            function: "main".into(),
            effect: Effect::FsRead,
        });

        let result = solver.solve();

        let mut allowed = EffectSet::pure();
        allowed.insert(Effect::FsRead);
        allowed.insert(Effect::FsWrite);

        let errors = solver.check_manifest(&result, "main", &allowed);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_manifest_check_fails() {
        let mut solver = EffectSolver::new();
        solver.add_constraint(EffectConstraint::Performs {
            function: "main".into(),
            effect: Effect::NetEgress,
        });

        let result = solver.solve();

        // Manifest only allows fs_read
        let allowed = EffectSet::single(Effect::FsRead);
        let errors = solver.check_manifest(&result, "main", &allowed);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, EffectErrorKind::ManifestViolation);
    }

    #[test]
    fn test_stdlib_declarations_load() {
        let decls = stdlib_effect_declarations();
        assert!(decls.len() >= 10);
    }
}
