// rusage_helper "<shell command>": run the command with `sh -c`, wait for it, and print the resource usage of all
// waited-for descendants to stderr as one line:
//   rusage-helper: status=S user=U sys=Y maxrss_kb=K wall=W
// Used by scripts/wj/economics.wj when neither `/usr/bin/time -l` (macOS) nor GNU `time -v` is installed. Plain std
// plus getrusage(RUSAGE_CHILDREN) declared here, so it builds with `rustc rusage_helper.rs` and no crates.
use std::process::Command;
use std::time::Instant;

#[repr(C)]
struct Timeval {
    sec: i64,
    usec: i64,
}

#[repr(C)]
struct Rusage {
    utime: Timeval,
    stime: Timeval,
    maxrss: i64,
    rest: [i64; 13],
}

extern "C" {
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

const RUSAGE_CHILDREN: i32 = -1;

fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: rusage_helper \"<shell command>\"");
        std::process::exit(2)
    });
    let t0 = Instant::now();
    let status = Command::new("sh").arg("-c").arg(&cmd).status();
    let wall = t0.elapsed().as_secs_f64();
    let code = match status {
        Ok(s) => s.code().unwrap_or(128),
        Err(e) => {
            eprintln!("rusage_helper: cannot run sh: {e}");
            127
        }
    };
    let mut u = Rusage { utime: Timeval { sec: 0, usec: 0 }, stime: Timeval { sec: 0, usec: 0 }, maxrss: 0, rest: [0; 13] };
    // SAFETY: `u` is a valid, writable `struct rusage` (two timevals then longs on 64-bit Linux and macOS).
    unsafe {
        getrusage(RUSAGE_CHILDREN, &mut u);
    }
    let user = u.utime.sec as f64 + u.utime.usec as f64 / 1e6;
    let sys = u.stime.sec as f64 + u.stime.usec as f64 / 1e6;
    // ru_maxrss: kilobytes on Linux, bytes on macOS
    let kb = if cfg!(target_os = "macos") { u.maxrss / 1024 } else { u.maxrss };
    eprintln!("rusage-helper: status={code} user={user:.3} sys={sys:.3} maxrss_kb={kb} wall={wall:.4}");
    std::process::exit(code);
}
