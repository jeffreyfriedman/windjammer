//! Toolchain discovery and the explicit skip policy.
//!
//! A missing toolchain is never silent: callers record a `Skip` and the report prints a loud
//! summary. `WJ_REQUIRE_TOOLCHAINS` (read here, in test code only) turns skips into failures.

use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tool {
    Rustc,
    Go,
    Node,
    Python3,
}

impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Tool::Rustc => "rustc",
            Tool::Go => "go",
            Tool::Node => "node",
            Tool::Python3 => "python3",
        }
    }

    fn version_arg(self) -> &'static str {
        match self {
            Tool::Go => "version",
            _ => "--version",
        }
    }

    pub fn available(self) -> bool {
        Command::new(self.name())
            .arg(self.version_arg())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// True when the environment demands that every toolchain be present (CI mode).
pub fn require_toolchains() -> bool {
    std::env::var("WJ_REQUIRE_TOOLCHAINS").map(|v| !v.is_empty() && v != "0").unwrap_or(false)
}

/// Result of probing the tools a test needs.
pub struct ToolProbe {
    pub missing: Vec<Tool>,
}

impl ToolProbe {
    pub fn new(needed: &[Tool]) -> Self {
        ToolProbe { missing: needed.iter().copied().filter(|t| !t.available()).collect() }
    }

    pub fn has(&self, t: Tool) -> bool {
        !self.missing.contains(&t)
    }

    /// Print a loud line per missing tool; panic when toolchains are required.
    pub fn announce(&self) {
        for t in &self.missing {
            eprintln!("CONFORMANCE-SKIP: toolchain `{}` not found; dependent cases are SKIPPED", t.name());
        }
        if !self.missing.is_empty() && require_toolchains() {
            panic!("WJ_REQUIRE_TOOLCHAINS is set but missing: {:?}", self.missing);
        }
    }
}
