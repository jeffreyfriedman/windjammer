// CLI module for unified wj command
//
// This module provides the implementation for all CLI subcommands.

pub mod add;
pub mod agent_wrap;
pub mod build;
pub mod build_caps;
pub mod build_json;
pub mod cache;
pub mod caps;
pub mod code_mode;
pub mod mcp_bind;
pub mod check;
pub mod clean;
pub mod errors;
pub mod eval;
pub mod explain_effects;
pub mod explain_ownership;
pub mod fix;
pub mod fmt;
pub mod crate_host_build;
pub mod host_build;
pub mod lint;
pub mod eject;
pub mod new;
mod new_templates;
pub mod reduce;
pub mod replay;
pub mod lock;
pub mod remove;
pub mod run;
pub mod rust_surface;
pub mod run_dev;
pub mod self_install;
pub mod speculate;
pub mod test;
pub mod tmp_janitor;
pub mod update;
pub mod verdict;
pub mod verify;
pub mod watch;
pub mod silenced;
pub mod widths_gate;

