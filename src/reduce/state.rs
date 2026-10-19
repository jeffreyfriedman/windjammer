//! Resumable state: the current best project and the oracle result cache, kept outside the output
//! directory (it holds un-anonymized source).

use super::project::Project;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    /// Hash of the original input; a state for different input is ignored.
    pub input_hash: String,
    pub oracle: String,
    pub seed: u64,
    pub round: usize,
    pub steps: usize,
    pub finished: bool,
    pub cache: HashMap<String, bool>,
}

pub fn dir_for(out: &Path) -> PathBuf {
    let mut name = out
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".reduce-state");
    out.with_file_name(name)
}

pub fn load(dir: &Path, input_hash: &str, oracle: &str, seed: u64) -> Option<(State, Project)> {
    let st: State =
        serde_json::from_str(&std::fs::read_to_string(dir.join("state.json")).ok()?).ok()?;
    if st.input_hash != input_hash || st.oracle != oracle || st.seed != seed {
        return None;
    }
    let project = Project::load(&dir.join("current")).ok()?;
    Some((st, project))
}

pub fn save(dir: &Path, st: &State, project: &Project) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let cur = dir.join("current");
    let _ = std::fs::remove_dir_all(&cur);
    project.write_to(&cur)?;
    std::fs::write(
        dir.join("state.json"),
        serde_json::to_string(st).unwrap_or_default(),
    )
}

pub fn cache_to_state(c: &HashMap<u64, bool>) -> HashMap<String, bool> {
    c.iter().map(|(k, v)| (format!("{k:016x}"), *v)).collect()
}

pub fn cache_from_state(c: &HashMap<String, bool>) -> HashMap<u64, bool> {
    c.iter()
        .filter_map(|(k, v)| u64::from_str_radix(k, 16).ok().map(|k| (k, *v)))
        .collect()
}
