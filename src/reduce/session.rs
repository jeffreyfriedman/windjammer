//! Reduction session: the current best project plus the engine and RNG that drive passes.

use super::engine::Engine;
use super::project::Project;
use super::rng::Rng;

pub struct Session<'a> {
    pub eng: &'a Engine,
    pub project: Project,
    pub rng: Rng,
}

impl<'a> Session<'a> {
    pub fn new(eng: &'a Engine, project: Project, seed: u64) -> Self {
        Session {
            eng,
            project,
            rng: Rng::new(seed),
        }
    }

    /// Files largest first; ties broken by the seeded shuffle.
    pub fn file_order(&mut self) -> Vec<String> {
        let mut names: Vec<String> = self.project.files.keys().cloned().collect();
        self.rng.shuffle(&mut names);
        names.sort_by_key(|n| std::cmp::Reverse(self.project.files[n].len()));
        names
    }

    /// Adopt the first interesting candidate; returns its index.
    pub fn adopt_first(&mut self, cands: Vec<Project>) -> Option<usize> {
        let k = self.eng.first_success(&cands)?;
        self.project = cands.into_iter().nth(k)?;
        Some(k)
    }
}
