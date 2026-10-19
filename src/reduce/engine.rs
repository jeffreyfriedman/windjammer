//! Candidate evaluation: validity filter (`wj check` class), content-hash cache, oracle, budget.
//!
//! Determinism: `first_success` evaluates candidates in parallel but always reports the lowest-index
//! success, so the result never depends on thread timing or on `--jobs`.

use super::oracle::{Oracle, OracleEnv};
use super::project::Project;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

pub struct Engine {
    pub oracle: Oracle,
    pub env: OracleEnv,
    pub jobs: usize,
    pub max_steps: usize,
    pub deadline: Instant,
    allowed_codes: BTreeSet<String>,
    cache: Mutex<HashMap<u64, bool>>,
    check_lock: Mutex<()>,
    steps: AtomicUsize,
    cache_hits: AtomicUsize,
    rejected: AtomicUsize,
    free_workers: Mutex<Vec<usize>>,
    unlimited: AtomicBool,
}

/// Error codes of `wj check` for `p` (`None` when the checker panics).
pub fn check_error_codes(p: &Project) -> Option<BTreeSet<String>> {
    let sources: Vec<(String, String)> = p
        .files
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let r = std::panic::catch_unwind(|| crate::check::check_sources(Path::new("."), &sources));
    let diags = r.ok()?;
    Some(
        diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| d.code.clone())
            .collect(),
    )
}

impl Engine {
    pub fn new(
        oracle: Oracle,
        env: OracleEnv,
        jobs: usize,
        max_steps: usize,
        deadline: Instant,
        baseline: &Project,
    ) -> Engine {
        let allowed = check_error_codes(baseline).unwrap_or_default();
        Engine {
            oracle,
            env,
            jobs: jobs.max(1),
            max_steps,
            deadline,
            allowed_codes: allowed,
            cache: Mutex::new(HashMap::new()),
            check_lock: Mutex::new(()),
            steps: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            rejected: AtomicUsize::new(0),
            free_workers: Mutex::new((0..jobs.max(1)).rev().collect()),
            unlimited: AtomicBool::new(false),
        }
    }

    /// Lift the step/time budget (the final anonymization check must always complete).
    pub fn set_unlimited(&self) {
        self.unlimited.store(true, Ordering::Relaxed);
    }

    pub fn exhausted(&self) -> bool {
        if self.unlimited.load(Ordering::Relaxed) {
            return false;
        }
        self.steps.load(Ordering::Relaxed) >= self.max_steps || Instant::now() >= self.deadline
    }
    pub fn steps(&self) -> usize {
        self.steps.load(Ordering::Relaxed)
    }
    pub fn cache_hits(&self) -> usize {
        self.cache_hits.load(Ordering::Relaxed)
    }
    pub fn rejected(&self) -> usize {
        self.rejected.load(Ordering::Relaxed)
    }
    pub fn cache_snapshot(&self) -> HashMap<u64, bool> {
        self.cache.lock().expect("cache").clone()
    }
    pub fn preload_cache(&self, entries: HashMap<u64, bool>) {
        self.cache.lock().expect("cache").extend(entries);
    }

    /// Does the oracle hold on the unmodified input? (Checked once up front, uncached.)
    pub fn baseline_holds(&self, p: &Project) -> bool {
        self.steps.fetch_add(1, Ordering::Relaxed);
        self.oracle.holds(&self.env, p, 0)
    }

    fn valid(&self, p: &Project) -> bool {
        if p.files.is_empty() {
            return false;
        }
        let _g = self.check_lock.lock().expect("check lock");
        check_error_codes(p).is_some_and(|c| c.is_subset(&self.allowed_codes))
    }

    /// Candidate keeps the bug and stays in the baseline's diagnostic class.
    pub fn interesting(&self, p: &Project) -> bool {
        let key = p.hash();
        if let Some(&v) = self.cache.lock().expect("cache").get(&key) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return v;
        }
        let v = if !self.valid(p) {
            self.rejected.fetch_add(1, Ordering::Relaxed);
            false
        } else if self.exhausted() {
            return false;
        } else {
            self.steps.fetch_add(1, Ordering::Relaxed);
            let worker = self
                .free_workers
                .lock()
                .expect("workers")
                .pop()
                .unwrap_or(0);
            let held = self.oracle.holds(&self.env, p, worker);
            self.free_workers.lock().expect("workers").push(worker);
            held
        };
        self.cache.lock().expect("cache").insert(key, v);
        v
    }

    /// Index of the lowest-index interesting candidate, evaluating `jobs` at a time.
    pub fn first_success(&self, cands: &[Project]) -> Option<usize> {
        for (ci, chunk) in cands.chunks(self.jobs).enumerate() {
            if self.exhausted() {
                return None;
            }
            let results: Vec<bool> = if chunk.len() == 1 {
                vec![self.interesting(&chunk[0])]
            } else {
                std::thread::scope(|s| {
                    let hs: Vec<_> = chunk
                        .iter()
                        .map(|c| s.spawn(move || self.interesting(c)))
                        .collect();
                    hs.into_iter().map(|h| h.join().unwrap_or(false)).collect()
                })
            };
            if let Some(k) = results.iter().position(|&b| b) {
                return Some(ci * self.jobs + k);
            }
        }
        None
    }
}
