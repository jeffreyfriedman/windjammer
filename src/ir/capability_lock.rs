//! Capability lock file (WJ-SEC-03).
//!
//! After solving, the per-function effect sets are serialized to
//! `.wj-capabilities.lock`. On subsequent builds, the lock file is compared
//! against the new effect sets. If a dependency upgrade adds new effects,
//! the build fails and requires explicit approval.

use crate::ir::safety_type::EffectSet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// On-disk representation of the capability lock file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityLockFile {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Windjammer compiler version that generated this lock file.
    pub windjammer_version: String,
    /// Per-function resolved effect sets.
    pub functions: BTreeMap<String, FunctionCapabilities>,
}

/// Capabilities for a single function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCapabilities {
    /// The effects this function performs (directly or transitively).
    pub effects: BTreeSet<String>,
}

/// Result of comparing a new solve result against the lock file.
#[derive(Debug)]
pub struct EscalationReport {
    /// Functions that gained new effects since the lock was generated.
    pub escalations: Vec<Escalation>,
}

/// A single escalation: a function gained a new effect.
#[derive(Debug, Clone)]
pub struct Escalation {
    pub function: String,
    pub new_effects: Vec<String>,
}

impl CapabilityLockFile {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            windjammer_version: env!("CARGO_PKG_VERSION").to_string(),
            functions: BTreeMap::new(),
        }
    }

    /// Build a lock file from solved effect sets.
    pub fn from_effect_sets(effect_sets: &std::collections::HashMap<String, EffectSet>) -> Self {
        let mut lock = Self::new();
        for (fn_name, effects) in effect_sets {
            let effect_strings: BTreeSet<String> =
                effects.iter().map(|e| e.to_string()).collect();
            if !effect_strings.is_empty() {
                lock.functions.insert(
                    fn_name.clone(),
                    FunctionCapabilities {
                        effects: effect_strings,
                    },
                );
            }
        }
        lock
    }

    /// Load a lock file from disk.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read lock file {}: {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse lock file {}: {}", path.display(), e))
    }

    /// Save the lock file to disk.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize lock file: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write lock file {}: {}", path.display(), e))
    }

    /// Compare a new set of effect results against this lock file.
    /// Returns escalations for any function that gained new effects.
    pub fn detect_escalations(
        &self,
        new_effects: &std::collections::HashMap<String, EffectSet>,
    ) -> EscalationReport {
        let mut escalations = Vec::new();

        for (fn_name, new_effect_set) in new_effects {
            let new_strings: BTreeSet<String> =
                new_effect_set.iter().map(|e| e.to_string()).collect();

            match self.functions.get(fn_name) {
                Some(locked) => {
                    let added: Vec<String> = new_strings
                        .difference(&locked.effects)
                        .cloned()
                        .collect();
                    if !added.is_empty() {
                        escalations.push(Escalation {
                            function: fn_name.clone(),
                            new_effects: added,
                        });
                    }
                }
                None if !new_strings.is_empty() => {
                    escalations.push(Escalation {
                        function: fn_name.clone(),
                        new_effects: new_strings.into_iter().collect(),
                    });
                }
                None => {}
            }
        }

        EscalationReport { escalations }
    }

    /// Approve specific escalations by merging them into the lock file.
    pub fn approve_escalations(&mut self, escalations: &[Escalation]) {
        for esc in escalations {
            let entry = self
                .functions
                .entry(esc.function.clone())
                .or_insert_with(|| FunctionCapabilities {
                    effects: BTreeSet::new(),
                });
            for effect in &esc.new_effects {
                entry.effects.insert(effect.clone());
            }
        }
    }
}

impl EscalationReport {
    pub fn has_escalations(&self) -> bool {
        !self.escalations.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.escalations.is_empty() {
            return "No capability escalations detected.".to_string();
        }
        let mut lines = Vec::new();
        lines.push(format!(
            "Capability escalations detected in {} function(s):",
            self.escalations.len()
        ));
        for esc in &self.escalations {
            lines.push(format!(
                "  {} gained: {}",
                esc.function,
                esc.new_effects.join(", ")
            ));
        }
        lines.push(String::new());
        lines.push("Run `wj capabilities approve` to accept these changes.".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::safety_type::Effect;

    #[test]
    fn test_lock_file_roundtrip() {
        let mut lock = CapabilityLockFile::new();
        lock.functions.insert(
            "read_config".to_string(),
            FunctionCapabilities {
                effects: ["fs_read".to_string()].into_iter().collect(),
            },
        );

        let json = serde_json::to_string(&lock).unwrap();
        let loaded: CapabilityLockFile = serde_json::from_str(&json).unwrap();
        assert_eq!(
            loaded.functions.get("read_config"),
            lock.functions.get("read_config")
        );
    }

    #[test]
    fn test_no_escalation_when_unchanged() {
        let mut lock = CapabilityLockFile::new();
        lock.functions.insert(
            "fetch".to_string(),
            FunctionCapabilities {
                effects: ["net_egress".to_string()].into_iter().collect(),
            },
        );

        let mut current = std::collections::HashMap::new();
        let mut eff = EffectSet::pure();
        eff.insert(Effect::NetEgress);
        current.insert("fetch".to_string(), eff);

        let report = lock.detect_escalations(&current);
        assert!(!report.has_escalations());
    }

    #[test]
    fn test_escalation_detected_on_new_effect() {
        let mut lock = CapabilityLockFile::new();
        lock.functions.insert(
            "fetch".to_string(),
            FunctionCapabilities {
                effects: ["net_egress".to_string()].into_iter().collect(),
            },
        );

        let mut current = std::collections::HashMap::new();
        let mut eff = EffectSet::pure();
        eff.insert(Effect::NetEgress);
        eff.insert(Effect::FsWrite);
        current.insert("fetch".to_string(), eff);

        let report = lock.detect_escalations(&current);
        assert!(report.has_escalations());
        assert_eq!(report.escalations.len(), 1);
        assert_eq!(report.escalations[0].function, "fetch");
        assert!(report.escalations[0].new_effects.contains(&"fs_write".to_string()));
    }

    #[test]
    fn test_escalation_detected_on_new_function() {
        let lock = CapabilityLockFile::new();

        let mut current = std::collections::HashMap::new();
        let mut eff = EffectSet::pure();
        eff.insert(Effect::ProcessSpawn);
        current.insert("run_script".to_string(), eff);

        let report = lock.detect_escalations(&current);
        assert!(report.has_escalations());
        assert_eq!(report.escalations[0].function, "run_script");
    }

    #[test]
    fn test_approve_escalations() {
        let mut lock = CapabilityLockFile::new();
        lock.functions.insert(
            "fetch".to_string(),
            FunctionCapabilities {
                effects: ["net_egress".to_string()].into_iter().collect(),
            },
        );

        let escalations = vec![Escalation {
            function: "fetch".to_string(),
            new_effects: vec!["fs_write".to_string()],
        }];

        lock.approve_escalations(&escalations);

        let fetch = lock.functions.get("fetch").unwrap();
        assert!(fetch.effects.contains("net_egress"));
        assert!(fetch.effects.contains("fs_write"));
    }

    #[test]
    fn test_from_effect_sets() {
        let mut effects = std::collections::HashMap::new();
        let mut eff = EffectSet::pure();
        eff.insert(Effect::FsRead);
        eff.insert(Effect::NetEgress);
        effects.insert("handler".to_string(), eff);

        let lock = CapabilityLockFile::from_effect_sets(&effects);
        let handler = lock.functions.get("handler").unwrap();
        assert!(handler.effects.contains("fs_read"));
        assert!(handler.effects.contains("net_egress"));
    }
}
