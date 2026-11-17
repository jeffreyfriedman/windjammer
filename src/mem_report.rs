//! Build memory report (`wj build --mem-report`, or `WJ_MEM_REPORT=1`).
//!
//! Peak memory and wall time of `wj build` are product meters: a library must build on a modest laptop.
//! When the report is on, the build prints one line per pipeline phase boundary (and every
//! [`PROGRESS_EVERY`] files inside per-file loops) to stderr:
//!
//! ```text
//! [wj-mem] t_ms=1234 rss_mib=412 peak_mib=530 phase=step3 convergence done
//! ```
//!
//! Fields are space-separated `key=value` pairs in this fixed order; `phase=` is last and runs to the end of
//! the line. `rss_mib` is the current resident set, `peak_mib` the process high-water mark, both in MiB
//! (0 when the platform does not report it). The final line of a build has `phase=build end`. The format is
//! a stable interface: memory meters and ratchets parse it (`tests/library_build_memory.rs`).
//!
//! A library build also prints its deterministic work counters ([`Work`]) after its last phase:
//!
//! ```text
//! [wj-work] registry_entries_cloned=48210 trait_keys_scanned=9120
//! ```
//!
//! The report is a [`MemReport`] handle the build creates from its configuration and passes down (no process
//! state). Off by default; it never influences generated output.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Per-file loops report every this many files (plus the first and last).
pub const PROGRESS_EVERY: usize = 25;

/// Current and peak resident set size of this process, in bytes (0 when unknown).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemSample {
    pub rss_bytes: u64,
    pub peak_bytes: u64,
}

/// Read the current and peak resident set size.
pub fn sample() -> MemSample {
    let (rss, peak) = platform::rss_and_peak();
    MemSample { rss_bytes: rss, peak_bytes: peak.max(rss) }
}

/// The report line for `phase` (without the trailing newline).
pub fn format_line(elapsed_ms: u128, s: MemSample, phase: &str) -> String {
    const MIB: u64 = 1024 * 1024;
    format!(
        "[wj-mem] t_ms={} rss_mib={} peak_mib={} phase={}",
        elapsed_ms,
        s.rss_bytes / MIB,
        s.peak_bytes / MIB,
        phase
    )
}

/// Parse a report line back into (rss_mib, peak_mib, phase). Used by meters and tests.
pub fn parse_line(line: &str) -> Option<(u64, u64, String)> {
    let rest = line.trim().strip_prefix("[wj-mem] ")?;
    let (head, phase) = rest.split_once(" phase=")?;
    let mut rss = None;
    let mut peak = None;
    for kv in head.split(' ') {
        match kv.split_once('=')? {
            ("rss_mib", v) => rss = v.parse().ok(),
            ("peak_mib", v) => peak = v.parse().ok(),
            _ => {}
        }
    }
    Some((rss?, peak?, phase.to_string()))
}

/// Deterministic work counters: how much library-sized work a build did, independent of machine speed and
/// load. Wall time is noisy; these are what the build-cost ratchet asserts on. Printed with the memory report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Work {
    /// Signature entries copied when the shared library registry had to be copied to be written.
    RegistryEntriesCloned = 0,
    /// Trait-impl keys walked by trait write-backs over a registry.
    TraitKeysScanned = 1,
}

const WORK_NAMES: [&str; 2] = ["registry_entries_cloned", "trait_keys_scanned"];

/// The memory report and work counters of one build. Created from the build configuration
/// (`CompilerConfig::mem_report`) and passed down the pipeline.
#[derive(Debug)]
pub struct MemReport {
    /// Start of the build when the report is on; `None` when it is off.
    start: Option<Instant>,
    work: [AtomicU64; 2],
}

impl MemReport {
    /// A report that prints when `enabled` (counters are kept either way).
    pub fn new(enabled: bool) -> Self {
        MemReport { start: enabled.then(Instant::now), work: [AtomicU64::new(0), AtomicU64::new(0)] }
    }

    /// True when the report prints.
    pub fn enabled(&self) -> bool {
        self.start.is_some()
    }

    /// Print a report line for a phase boundary (no-op when the report is off).
    pub fn phase(&self, name: &str) {
        if let Some(start) = self.start {
            eprintln!("{}", format_line(start.elapsed().as_millis(), sample(), name));
        }
    }

    /// Inside a per-file loop: report after file `done` (1-based) of `total` every [`PROGRESS_EVERY`] files.
    pub fn progress(&self, label: &str, done: usize, total: usize) {
        if self.enabled() && (done == 1 || done == total || done % PROGRESS_EVERY == 0) {
            self.phase(&format!("{label} {done}/{total}"));
        }
    }

    /// Add `n` units of `kind`.
    pub fn count(&self, kind: Work, n: u64) {
        self.work[kind as usize].fetch_add(n, Ordering::Relaxed);
    }

    /// The current value of a work counter.
    pub fn work(&self, kind: Work) -> u64 {
        self.work[kind as usize].load(Ordering::Relaxed)
    }

    /// `[wj-work] registry_entries_cloned=N trait_keys_scanned=N`
    pub fn work_line(&self) -> String {
        let fields: Vec<String> =
            WORK_NAMES.iter().zip(self.work.iter()).map(|(n, c)| format!("{n}={}", c.load(Ordering::Relaxed))).collect();
        format!("[wj-work] {}", fields.join(" "))
    }

    /// Print the work counters (no-op when the report is off).
    pub fn report_work(&self) {
        if self.enabled() {
            eprintln!("{}", self.work_line());
        }
    }
}

/// Parse a work line back into (name, value) pairs.
pub fn parse_work_line(line: &str) -> Option<Vec<(String, u64)>> {
    let rest = line.trim().strip_prefix("[wj-work] ")?;
    rest.split(' ').map(|kv| kv.split_once('=').and_then(|(k, v)| Some((k.to_string(), v.parse().ok()?)))).collect()
}

#[cfg(target_os = "linux")]
mod platform {
    /// VmRSS and VmHWM from /proc/self/status.
    pub fn rss_and_peak() -> (u64, u64) {
        let Ok(text) = std::fs::read_to_string("/proc/self/status") else {
            return (0, super::rusage_peak());
        };
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key))
                .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
                .map(|kb| kb * 1024)
                .unwrap_or(0)
        };
        (field("VmRSS:"), field("VmHWM:"))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    // mach_task_basic_info (MACH_TASK_BASIC_INFO = 20); time_value_t is two 32-bit integers.
    #[repr(C)]
    struct MachTaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_time: [i32; 2],
        system_time: [i32; 2],
        policy: i32,
        suspend_count: i32,
    }
    extern "C" {
        static mach_task_self_: u32;
        fn task_info(task: u32, flavor: u32, info: *mut i32, count: *mut u32) -> i32;
    }
    const MACH_TASK_BASIC_INFO: u32 = 20;

    pub fn rss_and_peak() -> (u64, u64) {
        let mut info = std::mem::MaybeUninit::<MachTaskBasicInfo>::zeroed();
        let mut count = (std::mem::size_of::<MachTaskBasicInfo>() / 4) as u32;
        // SAFETY: task_info writes at most `count` 32-bit words into `info`, which is that large.
        let kr = unsafe {
            task_info(mach_task_self_, MACH_TASK_BASIC_INFO, info.as_mut_ptr() as *mut i32, &mut count)
        };
        if kr != 0 {
            return (0, super::rusage_peak());
        }
        // SAFETY: task_info succeeded and filled the struct (it was zeroed beforehand).
        let info = unsafe { info.assume_init() };
        (info.resident_size, info.resident_size_max)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod platform {
    /// Portable fallback: only the high-water mark is known; report it for both.
    pub fn rss_and_peak() -> (u64, u64) {
        let peak = super::rusage_peak();
        (peak, peak)
    }
}

/// Peak RSS from getrusage (bytes on macOS, KiB elsewhere); 0 when unavailable.
#[cfg(all(unix, not(target_arch = "wasm32")))]
fn rusage_peak() -> u64 {
    // SAFETY: getrusage fills a plain struct; the pointer is valid for the call.
    unsafe {
        let mut ru: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut ru) != 0 {
            return 0;
        }
        #[cfg(target_os = "macos")]
        let bytes = ru.ru_maxrss as u64;
        #[cfg(not(target_os = "macos"))]
        let bytes = (ru.ru_maxrss as u64) * 1024;
        bytes
    }
}

#[cfg(not(all(unix, not(target_arch = "wasm32"))))]
fn rusage_peak() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_round_trips_through_the_parser() {
        let s = MemSample { rss_bytes: 412 * 1024 * 1024, peak_bytes: 530 * 1024 * 1024 };
        let line = format_line(1234, s, "step3 convergence done");
        assert_eq!(line, "[wj-mem] t_ms=1234 rss_mib=412 peak_mib=530 phase=step3 convergence done");
        assert_eq!(parse_line(&line), Some((412, 530, "step3 convergence done".to_string())));
        assert_eq!(parse_line("warning: something else"), None);
    }

    #[test]
    fn work_line_round_trips_through_the_parser() {
        let line = "[wj-work] registry_entries_cloned=12 trait_keys_scanned=3";
        assert_eq!(
            parse_work_line(line),
            Some(vec![("registry_entries_cloned".into(), 12), ("trait_keys_scanned".into(), 3)])
        );
        let report = MemReport::new(false);
        report.count(Work::TraitKeysScanned, 5);
        assert_eq!(report.work(Work::TraitKeysScanned), 5);
        assert!(!report.enabled());
        assert_eq!(parse_work_line(&report.work_line()), Some(vec![("registry_entries_cloned".into(), 0), ("trait_keys_scanned".into(), 5)]));
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn sample_reports_a_live_resident_set() {
        let s = sample();
        assert!(s.rss_bytes > 1024 * 1024, "rss {s:?}");
        assert!(s.peak_bytes >= s.rss_bytes, "peak {s:?}");
    }
}
