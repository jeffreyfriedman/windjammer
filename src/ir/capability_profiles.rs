//! Preset capability profiles for common package categories (WJ-SEC-04).
//!
//! Profiles provide sensible default effect allowances for packages based on
//! their intended role. The compiler can suggest or enforce a profile when
//! resolving dependencies from the package registry.

use crate::ir::safety_type::{Effect, EffectSet};

/// Trust level assigned to a package or profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrustLevel {
    /// First-party or audited packages (stdlib, official tools).
    Trusted,
    /// Cryptographically signed packages from verified publishers.
    Verified,
    /// Community packages with basic vetting.
    Community,
    /// Unknown or unvetted packages — most restrictive defaults apply.
    Unknown,
}

/// A preset capability profile describing allowed effects for a package category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProfile {
    pub name: &'static str,
    pub description: &'static str,
    pub allowed_effects: Vec<Effect>,
    pub trust_level: TrustLevel,
}

impl CapabilityProfile {
    /// Convert the profile's allowed effects into an `EffectSet`.
    pub fn to_effect_set(&self) -> EffectSet {
        EffectSet::from_iter(self.allowed_effects.clone())
    }

    /// Check whether a given effect is allowed by this profile.
    pub fn allows(&self, effect: &Effect) -> bool {
        self.allowed_effects.contains(effect)
    }
}

/// Pure computation — no side effects (parsers, serializers, math libraries).
pub fn parser() -> CapabilityProfile {
    CapabilityProfile {
        name: "parser",
        description: "Pure computation with no side effects",
        allowed_effects: vec![],
        trust_level: TrustLevel::Verified,
    }
}

/// HTTP client libraries — network egress only.
pub fn http_client() -> CapabilityProfile {
    CapabilityProfile {
        name: "http_client",
        description: "HTTP/network client libraries",
        allowed_effects: vec![Effect::NetEgress],
        trust_level: TrustLevel::Community,
    }
}

/// Database drivers — network egress and filesystem read.
pub fn database() -> CapabilityProfile {
    CapabilityProfile {
        name: "database",
        description: "Database drivers and ORM layers",
        allowed_effects: vec![Effect::NetEgress, Effect::FsRead],
        trust_level: TrustLevel::Community,
    }
}

/// File processors — read and write local files.
pub fn file_processor() -> CapabilityProfile {
    CapabilityProfile {
        name: "file_processor",
        description: "File processing and transformation tools",
        allowed_effects: vec![Effect::FsRead, Effect::FsWrite],
        trust_level: TrustLevel::Community,
    }
}

/// Build tools — filesystem, process spawning, and environment access.
pub fn build_tool() -> CapabilityProfile {
    CapabilityProfile {
        name: "build_tool",
        description: "Build and compilation tooling",
        allowed_effects: vec![
            Effect::FsRead,
            Effect::FsWrite,
            Effect::ProcessSpawn,
            Effect::EnvRead,
        ],
        trust_level: TrustLevel::Trusted,
    }
}

/// All built-in profiles, in declaration order.
pub fn all_profiles() -> Vec<CapabilityProfile> {
    vec![
        parser(),
        http_client(),
        database(),
        file_processor(),
        build_tool(),
    ]
}

/// Look up a built-in profile by name.
pub fn profile_by_name(name: &str) -> Option<CapabilityProfile> {
    all_profiles().into_iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_profile_is_pure() {
        let profile = parser();
        assert!(profile.to_effect_set().is_pure());
        assert_eq!(profile.trust_level, TrustLevel::Verified);
    }

    #[test]
    fn http_client_allows_net_egress_only() {
        let profile = http_client();
        assert!(profile.allows(&Effect::NetEgress));
        assert!(!profile.allows(&Effect::FsWrite));
    }

    #[test]
    fn build_tool_allows_process_spawn() {
        let profile = build_tool();
        assert!(profile.allows(&Effect::ProcessSpawn));
        assert!(profile.allows(&Effect::EnvRead));
    }

    #[test]
    fn profile_lookup_by_name() {
        assert!(profile_by_name("database").is_some());
        assert!(profile_by_name("nonexistent").is_none());
    }
}
