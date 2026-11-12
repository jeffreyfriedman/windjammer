//! Run a closure with the process's stdout and stderr pointed at the null device (ADR-0033).
//!
//! The build pipeline narrates itself with `println!`/`eprintln!`; when it runs in-process (the width and
//! `String` predictions generate the program the way `wj build` does) the narration must not reach the
//! program's own output. Only the descriptors are swapped, so the closure's own prints vanish; nothing else
//! observes the swap because the caller runs before the program. Unix only: elsewhere `None` is returned and the
//! caller uses a child process, which is the same pipeline with the same silence.

/// `Some(f())` with stdout and stderr silenced, or `None` when the platform cannot do it (`f` is not run).
#[cfg(unix)]
pub fn silenced<T>(f: impl FnOnce() -> T) -> Option<T> {
    use std::io::Write;
    use std::sync::Mutex;
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _held = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // SAFETY: plain descriptor duplication; every descriptor opened here is closed on every path below.
    unsafe {
        let null = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
        if null < 0 {
            return None;
        }
        let (out, err) = (libc::dup(1), libc::dup(2));
        if out < 0 || err < 0 {
            libc::close(null);
            for fd in [out, err] {
                if fd >= 0 {
                    libc::close(fd);
                }
            }
            return None;
        }
        libc::dup2(null, 1);
        libc::dup2(null, 2);
        libc::close(null);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        libc::dup2(out, 1);
        libc::dup2(err, 2);
        libc::close(out);
        libc::close(err);
        result.ok()
    }
}

#[cfg(not(unix))]
pub fn silenced<T>(_f: impl FnOnce() -> T) -> Option<T> {
    None
}
