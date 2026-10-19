//! The effect vocabulary of ADR-0021 (`fs.read`, `net.connect`, ...).
//!
//! Names are dotted and stable: they appear in manifests (`wj.toml [capabilities]`), diagnostics,
//! `wj explain-effects` output, the effect log and `@uses(...)` attributes.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EffectKind {
    FsRead,
    FsWrite,
    NetConnect,
    NetListen,
    EnvRead,
    EnvWrite,
    Process,
    Time,
    Random,
    Console,
    Ffi,
    /// `@effect("ledger.post")`
    User(String),
}

impl EffectKind {
    pub const BUILTIN: [EffectKind; 11] = [
        EffectKind::FsRead,
        EffectKind::FsWrite,
        EffectKind::NetConnect,
        EffectKind::NetListen,
        EffectKind::EnvRead,
        EffectKind::EnvWrite,
        EffectKind::Process,
        EffectKind::Time,
        EffectKind::Random,
        EffectKind::Console,
        EffectKind::Ffi,
    ];

    pub fn name(&self) -> String {
        match self {
            EffectKind::FsRead => "fs.read".into(),
            EffectKind::FsWrite => "fs.write".into(),
            EffectKind::NetConnect => "net.connect".into(),
            EffectKind::NetListen => "net.listen".into(),
            EffectKind::EnvRead => "env.read".into(),
            EffectKind::EnvWrite => "env.write".into(),
            EffectKind::Process => "process".into(),
            EffectKind::Time => "time".into(),
            EffectKind::Random => "random".into(),
            EffectKind::Console => "console".into(),
            EffectKind::Ffi => "ffi".into(),
            EffectKind::User(n) => n.clone(),
        }
    }

    /// Built-in names only; user-defined effects are created by `@effect("..")`.
    pub fn parse_builtin(name: &str) -> Option<EffectKind> {
        Self::BUILTIN.iter().find(|k| k.name() == name).cloned()
    }

    /// `time`, `random`, `console`: harmless for confinement, visible to durable/replay (ADR-0021 section 2).
    pub fn is_determinism(&self) -> bool {
        matches!(self, EffectKind::Time | EffectKind::Random | EffectKind::Console)
    }

    /// Granted unless the manifest denies it (decision D5).
    pub fn default_granted(&self) -> bool {
        self.is_determinism()
    }
}

impl fmt::Display for EffectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for k in EffectKind::BUILTIN {
            assert_eq!(EffectKind::parse_builtin(&k.name()), Some(k.clone()));
        }
        assert_eq!(EffectKind::parse_builtin("ledger.post"), None);
    }

    #[test]
    fn only_determinism_effects_are_granted_by_default() {
        let granted: Vec<String> = EffectKind::BUILTIN.iter().filter(|k| k.default_granted()).map(|k| k.name()).collect();
        assert_eq!(granted, ["time", "random", "console"]);
    }
}
