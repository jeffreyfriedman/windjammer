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
use windjammer::ir::effect_facts::grant::{DirAllOp, EntryOp, OpenError};
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
    fn write_grant(&self) -> Grant {
        let mut g = Grant::sandbox(&self.dir);
        g.allow(EffectKind::FsRead, "./granted/**");
        g.allow(EffectKind::FsWrite, "./granted/**");
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

/// Writes decide on the parent directory's descriptor. The oracle is the denied directory: nothing a write, a `mkdir` or an
/// `unlink` does through the link may touch it.
fn run_writer(w: &World, act: &dyn Fn(&Grant, &str, &Path) -> Option<bool>) -> Tally {
    let g = w.write_grant();
    let stop = Arc::new(AtomicBool::new(false));
    let s = swapper(w.dir.clone(), stop.clone(), Duration::ZERO, Arc::new(Mutex::new(Vec::new())));
    let start = Instant::now();
    let mut t = Tally::default();
    let via = w.dir.join("l").to_string_lossy().to_string();
    while start.elapsed() < STRESS {
        // Some(true): the action ran; Some(false): denied; None: an I/O error. The caller reports a leak by returning Some(true)
        // only when the denied directory was touched (it checks that itself).
        match act(&g, &via, &w.dir) {
            Some(true) => t.leaks += 1,
            Some(false) => t.denied += 1,
            None => t.io += 1,
        }
        t.allowed_reads += 1;
    }
    stop.store(true, Ordering::Relaxed);
    s.join().unwrap();
    t
}

fn write_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    use std::io::Write;
    let target = format!("{via}/out.txt");
    match g.open_write(&target, false) {
        Ok(Some(mut f)) => {
            let _ = f.write_all(b"x");
            let leaked = dir.join("denied/out.txt").exists();
            let _ = std::fs::remove_file(dir.join("denied/out.txt"));
            let _ = std::fs::remove_file(dir.join("granted/out.txt"));
            if leaked { Some(true) } else { None }
        }
        Ok(None) => None,
        Err(OpenError::Denied(_)) => Some(false),
        Err(OpenError::Io(_)) => None,
    }
}

fn mkdir_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    let target = format!("{via}/newdir");
    match g.entry_op(&target, EntryOp::MakeDir) {
        Ok(_) => {
            let leaked = dir.join("denied/newdir").exists();
            let _ = std::fs::remove_dir(dir.join("denied/newdir"));
            let _ = std::fs::remove_dir(dir.join("granted/newdir"));
            if leaked { Some(true) } else { None }
        }
        Err(OpenError::Denied(_)) => Some(false),
        Err(OpenError::Io(_)) => None,
    }
}

fn unlink_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    // both directories hold a file of that name; the one in the denied directory must survive
    let _ = std::fs::write(dir.join("denied/victim.txt"), "keep");
    let _ = std::fs::write(dir.join("granted/victim.txt"), "go");
    let r = g.entry_op(&format!("{via}/victim.txt"), EntryOp::RemoveFile);
    let leaked = !dir.join("denied/victim.txt").exists();
    match r {
        Ok(_) if leaked => Some(true),
        Ok(_) => None,
        Err(OpenError::Denied(_)) => Some(false),
        Err(OpenError::Io(_)) => None,
    }
}

fn rename_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    // into the link: the file must land in `granted`, never in `denied`; out of the link: the denied victim must survive
    let _ = std::fs::write(dir.join("granted/src.txt"), "s");
    let _ = std::fs::write(dir.join("denied/victim.txt"), "keep");
    let _ = std::fs::write(dir.join("granted/victim.txt"), "go");
    let into = g.rename_op(&dir.join("granted/src.txt").to_string_lossy(), &format!("{via}/moved.txt"));
    let out = g.rename_op(&format!("{via}/victim.txt"), &dir.join("granted/elsewhere.txt").to_string_lossy());
    let leaked = dir.join("denied/moved.txt").exists() || !dir.join("denied/victim.txt").exists();
    let _ = std::fs::remove_file(dir.join("denied/moved.txt"));
    let _ = std::fs::remove_file(dir.join("granted/moved.txt"));
    let _ = std::fs::remove_file(dir.join("granted/elsewhere.txt"));
    if leaked {
        return Some(true);
    }
    match (into, out) {
        (Err(OpenError::Denied(_)), _) | (_, Err(OpenError::Denied(_))) => Some(false),
        _ => None,
    }
}

fn copy_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    // out of the link into `granted`: the denied content must never arrive; into the link: nothing may appear in `denied`
    let _ = std::fs::remove_file(dir.join("granted/copied.txt"));
    let from = g.copy_op(&format!("{via}/f.txt"), &dir.join("granted/copied.txt").to_string_lossy());
    let arrived = std::fs::read_to_string(dir.join("granted/copied.txt")).is_ok_and(|t| t == "DENIED-SECRET");
    let to = g.copy_op(&dir.join("granted/f.txt").to_string_lossy(), &format!("{via}/copied.txt"));
    let leaked = arrived || dir.join("denied/copied.txt").exists();
    let _ = std::fs::remove_file(dir.join("denied/copied.txt"));
    let _ = std::fs::remove_file(dir.join("granted/copied.txt"));
    if leaked {
        return Some(true);
    }
    match (from, to) {
        (Err(OpenError::Denied(_)), _) | (_, Err(OpenError::Denied(_))) => Some(false),
        _ => None,
    }
}

fn create_all_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    let r = g.dir_all_op(&format!("{via}/x/y"), DirAllOp::Create);
    let leaked = dir.join("denied/x").exists();
    let _ = std::fs::remove_dir_all(dir.join("denied/x"));
    let _ = std::fs::remove_dir_all(dir.join("granted/x"));
    if leaked {
        return Some(true);
    }
    match r {
        Err(OpenError::Denied(_)) => Some(false),
        _ => None,
    }
}

fn remove_all_through(g: &Grant, via: &str, dir: &Path) -> Option<bool> {
    for d in ["denied", "granted"] {
        let _ = std::fs::create_dir_all(dir.join(d).join("tree/sub"));
        let _ = std::fs::write(dir.join(d).join("tree/sub/f.txt"), "keep");
    }
    let r = g.dir_all_op(&format!("{via}/tree"), DirAllOp::Remove);
    let leaked = !dir.join("denied/tree/sub/f.txt").exists();
    let _ = std::fs::remove_dir_all(dir.join("denied/tree"));
    let _ = std::fs::remove_dir_all(dir.join("granted/tree"));
    if leaked {
        return Some(true);
    }
    match r {
        Err(OpenError::Denied(_)) => Some(false),
        _ => None,
    }
}

#[test]
fn rename_copy_and_directory_trees_never_touch_the_denied_directory() {
    let w = World::new("w2");
    type Act = dyn Fn(&Grant, &str, &Path) -> Option<bool>;
    for (name, act) in [("rename", &rename_through as &Act), ("copy", &copy_through), ("create_dir_all", &create_all_through), ("remove_dir_all", &remove_all_through)] {
        let t = run_writer(&w, act);
        eprintln!("fd-verified {name}: {t:?}");
        assert_eq!(t.leaks, 0, "a {name} reached the denied directory: {t:?}");
        assert!(t.denied > 50 && t.io > 50, "the swap never raced with the {name}s: {t:?}");
    }
}

#[test]
fn nothing_lands_in_the_denied_directory_through_a_swapped_link() {
    let w = World::new("w");
    for (name, act) in [("write", &write_through as &dyn Fn(&Grant, &str, &Path) -> Option<bool>), ("mkdir", &mkdir_through), ("unlink", &unlink_through)] {
        let t = run_writer(&w, act);
        eprintln!("fd-verified {name}: {t:?}");
        assert_eq!(t.leaks, 0, "a {name} reached the denied directory: {t:?}");
        assert!(t.denied > 50 && t.io > 50, "the swap never raced with the {name}s: {t:?}");
    }
}

/// The stress must see the old order for writes too: check the name, then write the name.
#[test]
fn the_stress_sees_a_write_that_checks_the_name_first() {
    let w = World::new("wname");
    let naive = |g: &Grant, via: &str, dir: &Path| -> Option<bool> {
        let target = format!("{via}/out.txt");
        if g.check(&EffectKind::FsWrite, Some(&target)).is_err() {
            return Some(false);
        }
        let _ = std::fs::write(&target, "x");
        let leaked = dir.join("denied/out.txt").exists();
        let _ = std::fs::remove_file(dir.join("denied/out.txt"));
        let _ = std::fs::remove_file(dir.join("granted/out.txt"));
        if leaked { Some(true) } else { None }
    };
    let mut leaks = 0;
    let deadline = Instant::now() + Duration::from_secs(20);
    while leaks == 0 && Instant::now() < deadline {
        leaks += run_writer(&w, &naive).leaks;
    }
    assert!(leaks > 0, "the naive order never leaked in 20 s: the stress test cannot see the race");
}

#[allow(dead_code)]
fn _unused(_: &Path) {}
