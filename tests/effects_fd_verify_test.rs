//! ADR-0021: a file read decides on the path of the OPEN FILE, so a link re-pointed by another actor between "check" and "open"
//! cannot send a read to a denied file; the names that are only checked (exists, metadata, ...) rely on a decision cache whose
//! staleness is bounded by its time-to-live.
//!
//! The oracle is the content of the files: the granted file says GRANTED, the denied one says DENIED-SECRET. A swapper thread
//! re-points a link between the two directories as fast as it can (create a temporary link, rename it over the link: the same
//! as `ln -sfn`, atomic) while the reader reads through the link. No probability argument: on the fd-verified path a single
//! successful read of DENIED-SECRET fails the test.
#![cfg(unix)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windjammer::ir::effect_facts::grant::OpenError;
use windjammer::ir::effect_facts::{EffectKind, Grant};

const STRESS: Duration = Duration::from_millis(1500);

struct World {
    dir: PathBuf,
}

impl World {
    fn new(tag: &str) -> World {
        let dir = std::env::temp_dir().join(format!("wj_fdv_{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("granted")).unwrap();
        std::fs::create_dir_all(dir.join("denied")).unwrap();
        std::fs::write(dir.join("granted/f.txt"), "GRANTED").unwrap();
        std::fs::write(dir.join("denied/f.txt"), "DENIED-SECRET").unwrap();
        std::os::unix::fs::symlink(dir.join("granted"), dir.join("l")).unwrap();
        // canonical form (macOS puts /var under /private)
        World { dir: std::fs::canonicalize(&dir).unwrap() }
    }
    fn grant(&self) -> Grant {
        let mut g = Grant::sandbox(&self.dir);
        g.allow(EffectKind::FsRead, "./granted/**");
        g
    }
    fn link_path(&self) -> String {
        self.dir.join("l/f.txt").to_string_lossy().to_string()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Re-point `l` between `granted` and `denied` until `stop`; `hold` is how long each state lasts. Each swap is recorded with the
/// time just before the rename began and just after it returned: the link points to the new target for certain from the second
/// time until the first time of the next swap.
fn swapper(dir: PathBuf, stop: Arc<AtomicBool>, hold: Duration, events: Arc<Mutex<Vec<(Instant, Instant, bool)>>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let tmp = dir.join("l.tmp");
        let mut to_denied = true;
        while !stop.load(Ordering::Relaxed) {
            let target = if to_denied { dir.join("denied") } else { dir.join("granted") };
            let _ = std::fs::remove_file(&tmp);
            std::os::unix::fs::symlink(&target, &tmp).unwrap();
            let before = Instant::now();
            std::fs::rename(&tmp, dir.join("l")).unwrap();
            events.lock().unwrap().push((before, Instant::now(), to_denied));
            to_denied = !to_denied;
            if !hold.is_zero() {
                std::thread::sleep(hold);
            }
        }
    })
}

#[derive(Default, Debug)]
struct Tally {
    allowed_reads: u64,
    leaks: u64,
    denied: u64,
    io: u64,
}

fn run_reader(w: &World, read: &dyn Fn(&Grant, &str) -> Result<String, OpenError>) -> Tally {
    let g = w.grant();
    let stop = Arc::new(AtomicBool::new(false));
    let s = swapper(w.dir.clone(), stop.clone(), Duration::ZERO, Arc::new(Mutex::new(Vec::new())));
    let start = Instant::now();
    let mut t = Tally::default();
    let path = w.link_path();
    while start.elapsed() < STRESS {
        match read(&g, &path) {
            Ok(text) if text == "GRANTED" => t.allowed_reads += 1,
            Ok(_) => t.leaks += 1,
            Err(OpenError::Denied(_)) => t.denied += 1,
            Err(OpenError::Io(_)) => t.io += 1,
        }
    }
    stop.store(true, Ordering::Relaxed);
    s.join().unwrap();
    t
}

fn read_through(g: &Grant, p: &str) -> Result<String, OpenError> {
    let mut f = g.open_read(p)?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(OpenError::Io)?;
    Ok(s)
}

#[test]
fn no_denied_read_succeeds_through_a_swapped_link() {
    let w = World::new("fd");
    let t = run_reader(&w, &read_through);
    eprintln!("fd-verified read: {t:?}");
    assert_eq!(t.leaks, 0, "a read of the denied file succeeded: {t:?}");
    assert!(t.allowed_reads > 100 && t.denied > 100, "the swap never raced with the reads: {t:?}");
}

/// The stress test must be able to see the bug it guards against: checking the NAME and then opening the name (the old order)
/// leaks the denied file under the same load.
#[test]
fn the_stress_sees_a_check_by_name_followed_by_an_open() {
    let w = World::new("name");
    let naive = |g: &Grant, p: &str| -> Result<String, OpenError> {
        g.check(&EffectKind::FsRead, Some(p)).map_err(OpenError::Denied)?;
        std::fs::read_to_string(p).map_err(OpenError::Io)
    };
    let mut leaks = 0;
    let deadline = Instant::now() + Duration::from_secs(20);
    while leaks == 0 && Instant::now() < deadline {
        leaks += run_reader(&w, &naive).leaks;
    }
    assert!(leaks > 0, "the naive order never leaked in 20 s: the stress test cannot see the race");
}

/// A name that is only checked (no open) is decided from a cache. After the link went to the denied directory, an `Ok` may be
/// served for at most the time-to-live: the entry that served it was made before the swap, and an entry older than the
/// time-to-live is not served.
#[test]
fn cached_checks_are_stale_for_at_most_the_ttl() {
    let w = World::new("ttl");
    let g = w.grant();
    let ttl = windjammer::ir::effect_facts::grant::decision_ttl();
    let stop = Arc::new(AtomicBool::new(false));
    let events = Arc::new(Mutex::new(Vec::new()));
    let s = swapper(w.dir.clone(), stop.clone(), Duration::from_millis(3), events.clone());
    let path = w.link_path();
    let start = Instant::now();
    let mut calls: Vec<(Instant, Instant, bool)> = Vec::new();
    while start.elapsed() < STRESS {
        let a = Instant::now();
        let ok = g.check(&EffectKind::FsRead, Some(&path)).is_ok();
        calls.push((a, Instant::now(), ok));
    }
    stop.store(true, Ordering::Relaxed);
    s.join().unwrap();
    let events = events.lock().unwrap().clone();
    let (mut checked, mut worst) = (0u64, Duration::ZERO);
    for (a, e, ok) in calls {
        if !ok {
            continue;
        }
        // the last swap that had returned before the call began; the link points to the denied directory for certain until the
        // next rename begins, so only calls that ended before that count
        let Some(i) = events.iter().rposition(|(_, after, _)| *after <= a) else { continue };
        let (_, t_swap, to_denied) = events[i];
        let next_begins = events.get(i + 1).map(|(before, _, _)| *before);
        if !to_denied || next_begins.map_or(false, |n| n <= e) {
            continue;
        }
        checked += 1;
        worst = worst.max(a - t_swap);
        assert!(a - t_swap < ttl, "an allowed check began {:?} after the link was pointed at the denied directory (ttl {ttl:?})", a - t_swap);
    }
    eprintln!("cached checks: {checked} allowed checks after a swap to the denied directory, the latest began {worst:?} after it (ttl {ttl:?})");
    assert!(checked > 0, "no allowed check fell into a denied interval: the test measured nothing");
}

#[allow(dead_code)]
fn _unused(_: &Path) {}
