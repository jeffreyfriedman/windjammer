//! Developer diagnostics switches (`WJ_DEBUG_*`, `*_TRACE`, `WJ_PROFILE`).
//!
//! These only control stderr tracing and never influence generated output, so they are
//! the one sanctioned place (besides `compiler_config`) that reads the environment.
//! Behavior-changing toggles belong in `CompilerConfig`, never here.

/// `WJ_PHASES=1`: `[phase] NAME N` on stderr, N the instructions this process has retired so far (the hardware
/// counter of `proc_pid_rusage`, macOS), so a run's startup splits into measured phases (ADR-0038 ladder P2).
/// Prints nothing elsewhere or when the variable is unset.
pub fn phase(name: &str) {
    if !enabled("WJ_PHASES") {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        // SAFETY: `info` is a zeroed `rusage_info_v4`, the buffer `RUSAGE_INFO_V4` asks the kernel to fill.
        let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
        let r = unsafe { libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V4, &mut info as *mut _ as *mut libc::rusage_info_t) };
        if r == 0 {
            eprintln!("[phase] {name} {}", info.ri_instructions);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = name;
}

/// True when the diagnostics variable is set to any value.
pub fn enabled(name: &str) -> bool {
    read(name).is_some()
}

/// The one read of the environment in this module (a name lookup and the prefix snapshot below both go through it).
/// `wasm32-unknown-unknown` has no environment and `vars_os` panics there, so the browser playground sees none.
fn all() -> impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)> {
    #[cfg(target_arch = "wasm32")]
    {
        Vec::<(std::ffi::OsString, std::ffi::OsString)>::new().into_iter()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::vars_os()
    }
}

/// One variable by name. This is `getenv` (`var_os`): it compares names in place and copies only the value found. It is
/// not `all().find(..)`: that copies EVERY variable (names and values, so the length of `PWD` and `OLDPWD`, hence the
/// checkout path, too) per call, and these switches are read thousands of times per run (about 6% of a cold `wj run`
/// of the agent-loop kernel, and the last per-character cost of the working directory's path in `--self-test-cwd`).
/// The first variable of that name wins in both.
fn read(name: &str) -> Option<std::ffi::OsString> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var_os(name)
    }
}

/// Every variable whose name starts with one of `prefixes`, sorted: what a cache key hashes besides its inputs.
pub fn snapshot_with_prefixes(prefixes: &[&str]) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = all()
        .map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned()))
        .filter(|(k, _)| prefixes.iter().any(|p| k.starts_with(p)))
        .collect();
    env.sort();
    env
}

/// True when the variable is set to exactly `value`.
pub fn equals(name: &str, value: &str) -> bool {
    read(name).is_some_and(|v| v == value)
}

/// True when the diagnostics variable is set to `1`, `true` or `yes`.
pub fn truthy(name: &str) -> bool {
    matches!(
        read(name).and_then(|v| v.into_string().ok()).as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

/// The analysis stages `WJ_STAGE_COUNTERS=1` counts (docs/design/shared-program-analysis.md section 2.2): how often each
/// runs in one process. A run of a stage appends its name to the file `WJ_STAGE_COUNTERS` names and nothing else happens, so counting
/// keeps no state and cannot change output; a reader counts the lines (tests/common/stages.rs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// `Parser::parse` calls (lexing included).
    Parse,
    /// Analyzer runs (`analyze_program_with_global_arc`): pass A1 and pass A2 together.
    AnalyzerPass,
    /// Analyzer results taken from the session instead of computed (`program_analysis`).
    AnalyzerReused,
    /// `UnifiedNumericInference::infer_program` calls.
    Numeric,
    /// Numeric inference results taken from the session.
    NumericReused,
    /// `IrPipeline::lower_to_ir` calls.
    IrLowering,
    /// Full copies of the stdlib `SignatureRegistry` (`SignatureRegistry::new`, the clone `analyze_program_with_global_signatures` makes).
    RegistryCopy,
    /// `local_mut::facts::Facts::build` calls.
    FactsBuild,
    /// `codegen::generate_rust_source` calls (check-profile Rust generation).
    RustGeneration,
}

impl Stage {
    /// The name the report and the pinned tests use.
    pub fn name(self) -> &'static str {
        match self {
            Stage::Parse => "parse",
            Stage::AnalyzerPass => "analyzer_pass",
            Stage::AnalyzerReused => "analyzer_reused",
            Stage::Numeric => "numeric",
            Stage::NumericReused => "numeric_reused",
            Stage::IrLowering => "ir_lowering",
            Stage::RegistryCopy => "registry_copy",
            Stage::FactsBuild => "facts_build",
            Stage::RustGeneration => "rust_generation",
        }
    }
}

/// One more run of `stage` in this process: `WJ_STAGE_COUNTERS=<file>` appends one line `<name>` to the file (a file, not
/// stderr: the check silences stderr around its work).
pub fn count(stage: Stage) {
    use std::io::Write;
    let Some(path) = read("WJ_STAGE_COUNTERS") else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", stage.name());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_lookup_agrees_with_a_scan_of_the_environment() {
        // `var_os` and the scan name the same variable (the first of that name), set or unset; a name no variable can have is unset
        for (k, _) in std::env::vars_os().take(64) {
            let Some(k) = k.to_str() else { continue };
            if k.is_empty() || k.contains('=') {
                continue;
            }
            assert_eq!(read(k), all().find(|(n, _)| n == k).map(|(_, v)| v), "{k}");
            assert!(enabled(k));
        }
        assert_eq!(read("WJ_DEBUG_NO_SUCH_SWITCH_X"), None);
        assert!(!enabled("WJ_DEBUG_NO_SUCH_SWITCH_X"));
        assert_eq!(read("A=B"), None);
        assert_eq!(read(""), None);
        assert_eq!(read("a\0b"), None);
    }
}
