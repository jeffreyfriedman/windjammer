//! `wj` links no heavy system library (ADR-0038 P2, startup): on macOS loading CoreFoundation alone cost 6.5M
//! instructions before `main` of every `wj run`, and nothing on the run path needs it (chrono's `clock` feature
//! brought it, for the local time zone the compiler never reads). This gate reads the binary's load commands
//! (`otool -L`; Linux: `ldd`) and fails on a framework or library outside the system's C runtime.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// macOS frameworks that must not be linked into `wj` (each initializes at process start).
const HEAVY_MACOS: &[&str] = &["CoreFoundation.framework", "SystemConfiguration.framework", "Security.framework", "Foundation.framework", "AppKit.framework"];

/// Linux libraries `wj` may link (the C runtime and its usual companions).
const ALLOWED_LINUX: &[&str] = &["linux-vdso", "libc.so", "libm.so", "libgcc_s.so", "libpthread.so", "libdl.so", "librt.so", "ld-linux", "libutil.so"];

/// The linked libraries of `binary` that this gate refuses (`None`: no tool to list them on this platform).
fn heavy_libs(binary: &str) -> Option<Vec<String>> {
    if cfg!(target_os = "macos") {
        let o = Command::new("otool").arg("-L").arg(binary).output().ok()?;
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        Some(text.lines().skip(1).filter(|l| HEAVY_MACOS.iter().any(|h| l.contains(h))).map(|l| l.trim().to_string()).collect())
    } else if cfg!(target_os = "linux") {
        let o = Command::new("ldd").arg(binary).output().ok()?;
        let text = String::from_utf8_lossy(&o.stdout).to_string();
        Some(text.lines().filter(|l| !l.trim().is_empty() && !ALLOWED_LINUX.iter().any(|a| l.contains(a))).map(|l| l.trim().to_string()).collect())
    } else {
        None
    }
}

#[test]
fn wj_links_no_heavy_system_library() {
    let Some(found) = heavy_libs(WJ) else { return };
    assert!(found.is_empty(), "wj links libraries that load at every start (see ADR-0038 P2):\n{}", found.join("\n"));
}

/// The gate can fail: a system binary that links CoreFoundation is reported.
#[test]
#[cfg(target_os = "macos")]
fn the_gate_reports_a_binary_that_links_core_foundation() {
    let found = heavy_libs("/usr/bin/defaults").expect("otool");
    assert!(found.iter().any(|l| l.contains("CoreFoundation.framework")), "{found:?}");
}
