//! Caller-controlled execution modes (WJ-CONC-01).
//!
//! Functions are NOT colored as async/sync. Instead, the caller chooses
//! the execution mode at each call site:
//!
//! ```windjammer
//! fn fetch_user(id: i32) -> User { ... }
//!
//! let user = fetch_user(1)         // sync — blocks until complete
//! let user = async fetch_user(1)   // async — returns Future<User>
//! let handle = spawn fetch_user(1) // spawn — returns JoinHandle<User>
//! ```
//!
//! The execution mode is a property of the call EXPRESSION, not the function
//! DECLARATION. This eliminates function coloring entirely.

use crate::ir::safety_type::{Effect, EffectSet, ExecutionMode};
use std::collections::HashMap;

/// A call site with its execution mode.
#[derive(Debug, Clone)]
pub struct CallSite {
    /// The function being called.
    pub callee: String,
    /// The execution mode chosen by the caller.
    pub mode: ExecutionMode,
    /// Location for diagnostics.
    pub location: CallLocation,
}

#[derive(Debug, Clone)]
pub struct CallLocation {
    pub file: String,
    pub line: usize,
    pub col: usize,
}

/// Execution mode constraint for validation.
#[derive(Debug, Clone)]
pub enum ExecutionConstraint {
    /// A call site uses a specific execution mode.
    CallMode {
        site: CallSite,
    },
    /// A function is marked as not suitable for spawning
    /// (e.g., it captures mutable references that can't be sent across threads).
    NotSpawnable {
        function: String,
        reason: String,
    },
    /// A function requires async context (it uses .await internally).
    RequiresAsyncContext {
        function: String,
    },
}

/// Validates execution mode choices at call sites.
pub struct ExecutionValidator {
    call_sites: Vec<CallSite>,
    not_spawnable: HashMap<String, String>,
    requires_async: HashMap<String, bool>,
    /// Effect sets for functions (from the effect solver).
    function_effects: HashMap<String, EffectSet>,
}

impl ExecutionValidator {
    pub fn new() -> Self {
        Self {
            call_sites: Vec::new(),
            not_spawnable: HashMap::new(),
            requires_async: HashMap::new(),
            function_effects: HashMap::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: ExecutionConstraint) {
        match constraint {
            ExecutionConstraint::CallMode { site } => {
                self.call_sites.push(site);
            }
            ExecutionConstraint::NotSpawnable { function, reason } => {
                self.not_spawnable.insert(function, reason);
            }
            ExecutionConstraint::RequiresAsyncContext { function } => {
                self.requires_async.insert(function, true);
            }
        }
    }

    /// Set function effects (from effect solver output).
    pub fn set_function_effects(&mut self, effects: HashMap<String, EffectSet>) {
        self.function_effects = effects;
    }

    /// Validate all call sites and return errors.
    pub fn validate(&self) -> ExecutionValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for site in &self.call_sites {
            match site.mode {
                ExecutionMode::Spawn => {
                    // Check if function is spawnable
                    if let Some(reason) = self.not_spawnable.get(&site.callee) {
                        errors.push(ExecutionError {
                            kind: ExecutionErrorKind::NotSpawnable,
                            call_site: site.clone(),
                            message: format!(
                                "cannot spawn '{}': {}",
                                site.callee, reason
                            ),
                        });
                    }

                    // Warn if spawning a pure/logic-only function (no benefit)
                    if let Some(effects) = self.function_effects.get(&site.callee) {
                        if effects.is_pure() {
                            warnings.push(ExecutionWarning {
                                call_site: site.clone(),
                                message: format!(
                                    "spawning pure function '{}' has no concurrency benefit",
                                    site.callee
                                ),
                            });
                        }
                    }
                }

                ExecutionMode::Async => {
                    // Async is always valid — it just wraps in a future
                }

                ExecutionMode::Sync => {
                    // Sync is always valid — default behavior
                }
            }
        }

        ExecutionValidationResult { errors, warnings }
    }

    /// Generate target-specific code hints for each call site.
    pub fn codegen_hints(&self) -> Vec<ExecutionCodegenHint> {
        self.call_sites
            .iter()
            .map(|site| ExecutionCodegenHint {
                callee: site.callee.clone(),
                mode: site.mode.clone(),
                rust_emit: match site.mode {
                    ExecutionMode::Sync => format!("{}(args)", site.callee),
                    ExecutionMode::Async => format!("{}(args).await", site.callee),
                    ExecutionMode::Spawn => {
                        format!("tokio::spawn(async move {{ {}(args) }})", site.callee)
                    }
                },
                go_emit: match site.mode {
                    ExecutionMode::Sync => format!("{}(args)", site.callee),
                    ExecutionMode::Async => format!("{}(args)", site.callee), // Go doesn't have async
                    ExecutionMode::Spawn => format!("go {}(args)", site.callee),
                },
                js_emit: match site.mode {
                    ExecutionMode::Sync => format!("{}(args)", site.callee),
                    ExecutionMode::Async => format!("await {}(args)", site.callee),
                    ExecutionMode::Spawn => {
                        format!("Promise.resolve().then(() => {}(args))", site.callee)
                    }
                },
            })
            .collect()
    }
}

/// Result of execution mode validation.
#[derive(Debug)]
pub struct ExecutionValidationResult {
    pub errors: Vec<ExecutionError>,
    pub warnings: Vec<ExecutionWarning>,
}

#[derive(Debug, Clone)]
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub call_site: CallSite,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionErrorKind {
    NotSpawnable,
    InvalidContext,
}

#[derive(Debug, Clone)]
pub struct ExecutionWarning {
    pub call_site: CallSite,
    pub message: String,
}

/// Code generation hint for a call site.
#[derive(Debug, Clone)]
pub struct ExecutionCodegenHint {
    pub callee: String,
    pub mode: ExecutionMode,
    pub rust_emit: String,
    pub go_emit: String,
    pub js_emit: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(line: usize) -> CallLocation {
        CallLocation {
            file: "test.wj".into(),
            line,
            col: 1,
        }
    }

    #[test]
    fn test_sync_always_valid() {
        let mut validator = ExecutionValidator::new();
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "fetch_user".into(),
                mode: ExecutionMode::Sync,
                location: loc(1),
            },
        });

        let result = validator.validate();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_async_always_valid() {
        let mut validator = ExecutionValidator::new();
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "fetch_user".into(),
                mode: ExecutionMode::Async,
                location: loc(1),
            },
        });

        let result = validator.validate();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_spawn_not_spawnable_error() {
        let mut validator = ExecutionValidator::new();
        validator.add_constraint(ExecutionConstraint::NotSpawnable {
            function: "mutate_state".into(),
            reason: "captures &mut reference".into(),
        });
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "mutate_state".into(),
                mode: ExecutionMode::Spawn,
                location: loc(5),
            },
        });

        let result = validator.validate();
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, ExecutionErrorKind::NotSpawnable);
    }

    #[test]
    fn test_spawn_pure_function_warning() {
        let mut validator = ExecutionValidator::new();

        let mut effects = HashMap::new();
        effects.insert("add_numbers".into(), EffectSet::pure());
        validator.set_function_effects(effects);

        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "add_numbers".into(),
                mode: ExecutionMode::Spawn,
                location: loc(1),
            },
        });

        let result = validator.validate();
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_no_function_coloring() {
        // The same function can be called in all three modes
        let mut validator = ExecutionValidator::new();
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "fetch_user".into(),
                mode: ExecutionMode::Sync,
                location: loc(1),
            },
        });
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "fetch_user".into(),
                mode: ExecutionMode::Async,
                location: loc(2),
            },
        });
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "fetch_user".into(),
                mode: ExecutionMode::Spawn,
                location: loc(3),
            },
        });

        let result = validator.validate();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_codegen_hints() {
        let mut validator = ExecutionValidator::new();
        validator.add_constraint(ExecutionConstraint::CallMode {
            site: CallSite {
                callee: "fetch_user".into(),
                mode: ExecutionMode::Spawn,
                location: loc(1),
            },
        });

        let hints = validator.codegen_hints();
        assert_eq!(hints.len(), 1);
        assert!(hints[0].rust_emit.contains("tokio::spawn"));
        assert!(hints[0].go_emit.contains("go "));
        assert!(hints[0].js_emit.contains("Promise"));
    }
}
