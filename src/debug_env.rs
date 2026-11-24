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

/// The one enumeration of the environment in this module (the prefix snapshot below goes through it).
/// `wasm32-unknown-unknown` has no environment and `vars_os` panics there, so the browser playground sees none.
fn all() -> impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)> {
    #[cfg(target_arch = "wasm32")]
    {
        Vec::<(std::ffi::OsString, std::ffi::OsString)>::new().into_iter()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::vars_os().collect::<Vec<_>>().into_iter()
    }
}

/// A name lookup does not enumerate the environment: `var_os` is safe on every target (`vars_os` panics on wasm32).
fn read(name: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(name)
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
