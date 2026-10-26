//! Resource limits of a sandboxed run (`--limits fuel=N,wall=SECS,output=BYTES,calls=N,mem=MB`, ADR-0021 section 12).
//!
//! * `fuel`   evaluated expressions in the tree walker (a limit forces the walker: the compiled engine does not tick)
//! * `wall`   seconds of real time, polled every 1024 steps
//! * `output` bytes written to standard output and error
//! * `calls`  gated effect calls (console output is not counted)
//! * `mem`    resident memory of the process in MiB, polled every 8192 steps (a coarse bound meant for a one-shot sandbox process)
//!
//! Exceeding one stops the run with exit status 124 and a message naming the limit. The error is not catchable by the program.

use super::engine::Interpreter;
use super::error::{Ctl, ErrorKind, RuntimeError};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LimitSpec {
    pub fuel: Option<u64>,
    pub wall: Option<Duration>,
    pub output: Option<usize>,
    pub calls: Option<usize>,
    pub mem_mb: Option<u64>,
}

impl LimitSpec {
    /// `fuel=1e8,wall=10,output=65536,calls=20,mem=256`
    pub fn parse(text: &str) -> Result<LimitSpec, String> {
        let mut s = LimitSpec::default();
        for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let (k, v) = part.split_once('=').ok_or_else(|| format!("--limits takes name=value pairs, found `{part}`"))?;
            let num = |v: &str| -> Result<f64, String> { v.trim().parse::<f64>().map_err(|_| format!("--limits {k}: `{v}` is not a number")) };
            match k.trim() {
                "fuel" => s.fuel = Some(num(v)? as u64),
                "wall" => s.wall = Some(Duration::from_secs_f64(num(v)?.max(0.0))),
                "output" => s.output = Some(num(v)? as usize),
                "calls" => s.calls = Some(num(v)? as usize),
                "mem" => s.mem_mb = Some(num(v)? as u64),
                other => return Err(format!("--limits: unknown limit `{other}` (known: fuel, wall, output, calls, mem)")),
            }
        }
        Ok(s)
    }
}

pub struct Limits {
    spec: LimitSpec,
    started: Instant,
    pub steps: u64,
    pub out_bytes: usize,
    pub calls_made: usize,
    exceeded: Option<String>,
}

/// Resident set size of this process in MiB (0 when unknown).
fn rss_mb() -> u64 {
    // SAFETY: getrusage fills a plain struct; the pointer is valid for the call.
    unsafe {
        let mut ru: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut ru) != 0 {
            return 0;
        }
        // ru_maxrss is bytes on macOS and kilobytes elsewhere
        #[cfg(target_os = "macos")]
        let bytes = ru.ru_maxrss as u64;
        #[cfg(not(target_os = "macos"))]
        let bytes = (ru.ru_maxrss as u64) * 1024;
        bytes / (1024 * 1024)
    }
}

fn limit_error(msg: String) -> Ctl {
    Ctl::Err(Box::new(RuntimeError::new(ErrorKind::Limit, msg)))
}

impl Interpreter {
    pub fn set_limits(&mut self, spec: LimitSpec) {
        self.limits = Some(Box::new(Limits { spec, started: Instant::now(), steps: 0, out_bytes: 0, calls_made: 0, exceeded: None }));
    }

    /// Steps, effect calls and output bytes used so far (for `--result-json`).
    pub fn usage(&self) -> (u64, usize, usize) {
        self.limits.as_ref().map_or((0, 0, 0), |l| (l.steps, l.calls_made, l.out_bytes))
    }

    /// One evaluated expression.
    #[inline]
    pub(crate) fn limit_tick(&mut self) -> Result<(), Ctl> {
        let Some(l) = self.limits.as_mut() else { return Ok(()) };
        l.steps += 1;
        if let Some(m) = &l.exceeded {
            return Err(limit_error(m.clone()));
        }
        if let Some(f) = l.spec.fuel {
            if l.steps > f {
                return Err(limit_error(format!("limit exceeded: fuel ({f} steps); raise it with --limits fuel=N")));
            }
        }
        if l.steps % 1024 == 0 {
            if let Some(w) = l.spec.wall {
                if l.started.elapsed() > w {
                    return Err(limit_error(format!("limit exceeded: wall time ({:.1}s); raise it with --limits wall=SECONDS", w.as_secs_f64())));
                }
            }
        }
        if l.steps % 8192 == 0 {
            if let Some(m) = l.spec.mem_mb {
                if rss_mb() > m {
                    return Err(limit_error(format!("limit exceeded: memory ({m} MiB); raise it with --limits mem=MIB")));
                }
            }
        }
        Ok(())
    }

    /// One gated effect call.
    pub(crate) fn limit_call(&mut self) -> Result<(), Ctl> {
        let Some(l) = self.limits.as_mut() else { return Ok(()) };
        l.calls_made += 1;
        match l.spec.calls {
            Some(c) if l.calls_made > c => Err(limit_error(format!("limit exceeded: effect calls ({c}); raise it with --limits calls=N"))),
            _ => Ok(()),
        }
    }

    /// Bytes the program wrote; the run stops at the next step once the limit is passed.
    pub(crate) fn limit_output(&mut self, n: usize) {
        if let Some(l) = self.limits.as_mut() {
            l.out_bytes += n;
            if let Some(o) = l.spec.output {
                if l.out_bytes > o && l.exceeded.is_none() {
                    l.exceeded = Some(format!("limit exceeded: output ({o} bytes); raise it with --limits output=BYTES"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_parse() {
        let s = LimitSpec::parse("fuel=1e6, wall=2.5,output=100,calls=3,mem=64").unwrap();
        assert_eq!((s.fuel, s.output, s.calls, s.mem_mb), (Some(1_000_000), Some(100), Some(3), Some(64)));
        assert_eq!(s.wall, Some(Duration::from_millis(2500)));
        assert!(LimitSpec::parse("fuel").is_err());
        assert!(LimitSpec::parse("speed=1").is_err());
        assert!(LimitSpec::parse("fuel=lots").is_err());
    }
}
