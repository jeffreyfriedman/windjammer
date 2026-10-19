//! In-memory project: relative path -> source text. Everything the reducer manipulates.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Project {
    pub files: BTreeMap<String, String>,
}

impl Project {
    /// Load a `.wj` file (single-file project named `main.wj`) or every `.wj` file under a directory.
    pub fn load(path: &Path) -> anyhow::Result<Project> {
        let mut files = BTreeMap::new();
        if path.is_file() {
            files.insert("main.wj".to_string(), std::fs::read_to_string(path)?);
        } else if path.is_dir() {
            for f in crate::module_system::sources::wj_files_under(path)? {
                let rel = f.strip_prefix(path).unwrap_or(&f);
                let key = rel.to_string_lossy().replace('\\', "/");
                files.insert(key, std::fs::read_to_string(&f)?);
            }
        } else {
            anyhow::bail!("path not found: {}", path.display());
        }
        if files.is_empty() {
            anyhow::bail!("no .wj files found in {}", path.display());
        }
        Ok(Project { files })
    }

    pub fn write_to(&self, dir: &Path) -> std::io::Result<()> {
        for (rel, text) in &self.files {
            let p: PathBuf = dir.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, text)?;
        }
        Ok(())
    }

    pub fn line_count(&self) -> usize {
        self.files
            .values()
            .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
            .sum()
    }

    pub fn byte_count(&self) -> usize {
        self.files.values().map(|t| t.len()).sum()
    }

    /// Stable content hash (FNV-1a 64) over sorted paths and contents.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mut eat = |bytes: &[u8]| {
            for b in bytes {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        };
        for (k, v) in &self.files {
            eat(k.as_bytes());
            eat(&[0]);
            eat(v.as_bytes());
            eat(&[1]);
        }
        h
    }

    pub fn with_file(&self, rel: &str, text: String) -> Project {
        let mut p = self.clone();
        p.files.insert(rel.to_string(), text);
        p
    }

    pub fn without_files(&self, drop: &[&str]) -> Project {
        let mut p = self.clone();
        for d in drop {
            p.files.remove(*d);
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_depends_on_paths_and_content() {
        let mut a = Project::default();
        a.files.insert("a.wj".into(), "x".into());
        let b = a.with_file("a.wj", "y".into());
        let c = a.with_file("b.wj", "x".into()).without_files(&["a.wj"]);
        assert_ne!(a.hash(), b.hash());
        assert_ne!(a.hash(), c.hash());
        assert_eq!(a.hash(), a.clone().hash());
    }
}
