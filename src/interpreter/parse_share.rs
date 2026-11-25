//! Parsed files shared between the tests of one `wj test --interpret` run (soundness:
//! `docs/design/interp-soundness/parseshare.md`). Every test runs in a fresh `Interpreter` that loads its whole project;
//! without this each test would lex and parse every file again (and leak each parse). The key is the file's content key
//! (`compiled_facts::content_key`: name, text, renames). It is passed down by the test runner, never a static.

use super::compiled_facts::{content_key, note_parsed};
use crate::parser::Program;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Mutex;

#[derive(Default)]
pub struct ParseShare {
    entries: Mutex<HashMap<String, &'static Program<'static>>>,
    hits: AtomicU64,
    misses: AtomicU64,
    mismatches: AtomicU64,
    /// Parse afresh on every hit and count a stored parse whose dump differs (the staleness gate).
    verify: bool,
}

impl ParseShare {
    pub fn new() -> Self {
        Self::default()
    }
    /// A share that re-parses every hit and counts a stored parse that differs from the fresh one.
    pub fn verifying() -> Self {
        ParseShare { verify: true, ..Self::default() }
    }
    /// (hits, misses): a miss is one parse run, a hit one skipped.
    pub fn counts(&self) -> (u64, u64) {
        (self.hits.load(Relaxed), self.misses.load(Relaxed))
    }
    /// Hits whose stored parse differed from a fresh one (only counted by a verifying share; must be 0).
    pub fn mismatches(&self) -> u64 {
        self.mismatches.load(Relaxed)
    }
    /// The `[fast]` line of `WJ_JIT_DEBUG` for a test run.
    pub fn line(&self) -> String {
        let (h, m) = self.counts();
        format!("[fast] parseshare hits {h} misses {m}")
    }

    /// The parse of `source` as `file` with `renames`: the shared one, or parsed and kept (a failure is not kept).
    pub fn parse(&self, source: &str, file: &str, renames: &HashMap<String, String>) -> Result<&'static Program<'static>, String> {
        let key = content_key(file, source, renames);
        let found = self.entries.lock().unwrap_or_else(|e| e.into_inner()).get(&key).copied();
        if let Some(p) = found {
            self.hits.fetch_add(1, Relaxed);
            note_parsed(file, source, renames);
            if self.verify {
                let fresh = super::loader::parse_source_renaming(source, file, renames)?;
                if crate::source_map::elided_debug(&fresh) != crate::source_map::elided_debug(p) {
                    self.mismatches.fetch_add(1, Relaxed);
                }
            }
            return Ok(p);
        }
        self.misses.fetch_add(1, Relaxed);
        let p: &'static Program<'static> = Box::leak(Box::new(super::loader::parse_source_renaming(source, file, renames)?));
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).insert(key, p);
        Ok(p)
    }
}
