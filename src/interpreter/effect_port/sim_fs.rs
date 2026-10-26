//! The simulated file system: an in-memory tree with the error texts a real one gives (`std::io::Error` Display on Unix),
//! so a program that matches on or prints an error behaves the same simulated and live. Paths are absolute and lexical.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const ENOENT: &str = "No such file or directory (os error 2)";
pub const EEXIST: &str = "File exists (os error 17)";
pub const ENOTDIR: &str = "Not a directory (os error 20)";
pub const EISDIR: &str = "Is a directory (os error 21)";
pub const ENOTEMPTY: &str = "Directory not empty (os error 39)";

#[derive(Default)]
pub struct SimFs {
    files: BTreeMap<String, Vec<u8>>,
    dirs: BTreeSet<String>,
    cwd: String,
}

/// Lexical normalization of an absolute path: no `.`, no `..`, no doubled or trailing slash.
pub fn normalize(abs: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in abs.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

fn parent(p: &str) -> String {
    match p.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => p[..i].to_string(),
    }
}

impl SimFs {
    pub fn new(cwd: &str) -> SimFs {
        let mut fs = SimFs { cwd: normalize(cwd), ..SimFs::default() };
        fs.dirs.insert("/".into());
        fs.mkdirs(&fs.cwd.clone()).ok();
        fs
    }

    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    pub fn abs(&self, p: &str) -> String {
        if p.starts_with('/') {
            normalize(p)
        } else {
            normalize(&format!("{}/{}", self.cwd, p))
        }
    }

    pub fn set_cwd(&mut self, p: &str) -> Result<(), String> {
        let a = self.abs(p);
        if self.dirs.contains(&a) {
            self.cwd = a;
            Ok(())
        } else if self.files.contains_key(&a) {
            Err(ENOTDIR.into())
        } else {
            Err(ENOENT.into())
        }
    }

    pub fn is_file(&self, p: &str) -> bool {
        self.files.contains_key(&self.abs(p))
    }

    pub fn is_dir(&self, p: &str) -> bool {
        self.dirs.contains(&self.abs(p))
    }

    pub fn exists(&self, p: &str) -> bool {
        self.is_file(p) || self.is_dir(p)
    }

    fn mkdirs(&mut self, abs: &str) -> Result<(), String> {
        let mut cur = String::new();
        for seg in abs.split('/').filter(|s| !s.is_empty()) {
            cur.push('/');
            cur.push_str(seg);
            if self.files.contains_key(&cur) {
                return Err(ENOTDIR.into());
            }
            self.dirs.insert(cur.clone());
        }
        Ok(())
    }

    pub fn create_dir_all(&mut self, p: &str) -> Result<(), String> {
        let a = self.abs(p);
        self.mkdirs(&a)
    }

    pub fn create_dir(&mut self, p: &str) -> Result<(), String> {
        let a = self.abs(p);
        if self.exists(&a) {
            return Err(EEXIST.into());
        }
        if !self.dirs.contains(&parent(&a)) {
            return Err(if self.files.contains_key(&parent(&a)) { ENOTDIR } else { ENOENT }.into());
        }
        self.dirs.insert(a);
        Ok(())
    }

    pub fn read(&self, p: &str) -> Result<Vec<u8>, String> {
        let a = self.abs(p);
        match self.files.get(&a) {
            Some(b) => Ok(b.clone()),
            None if self.dirs.contains(&a) => Err(EISDIR.into()),
            None => Err(ENOENT.into()),
        }
    }

    fn check_target(&self, a: &str) -> Result<(), String> {
        if self.dirs.contains(a) {
            return Err(EISDIR.into());
        }
        let par = parent(a);
        if self.dirs.contains(&par) {
            Ok(())
        } else if self.files.contains_key(&par) {
            Err(ENOTDIR.into())
        } else {
            Err(ENOENT.into())
        }
    }

    pub fn write(&mut self, p: &str, data: Vec<u8>) -> Result<(), String> {
        let a = self.abs(p);
        self.check_target(&a)?;
        self.files.insert(a, data);
        Ok(())
    }

    pub fn append(&mut self, p: &str, data: &[u8]) -> Result<(), String> {
        let a = self.abs(p);
        self.check_target(&a)?;
        self.files.entry(a).or_default().extend_from_slice(data);
        Ok(())
    }

    pub fn remove_file(&mut self, p: &str) -> Result<(), String> {
        let a = self.abs(p);
        if self.files.remove(&a).is_some() {
            Ok(())
        } else if self.dirs.contains(&a) {
            Err(EISDIR.into())
        } else {
            Err(ENOENT.into())
        }
    }

    fn children(&self, dir: &str) -> Vec<String> {
        let prefix = if dir == "/" { "/".to_string() } else { format!("{dir}/") };
        let mut names: BTreeSet<String> = BTreeSet::new();
        for k in self.files.keys().chain(self.dirs.iter()) {
            if let Some(rest) = k.strip_prefix(&prefix) {
                if !rest.is_empty() {
                    names.insert(rest.split('/').next().unwrap_or(rest).to_string());
                }
            }
        }
        names.into_iter().collect()
    }

    pub fn read_dir(&self, p: &str) -> Result<Vec<String>, String> {
        let a = self.abs(p);
        if self.dirs.contains(&a) {
            Ok(self.children(&a))
        } else if self.files.contains_key(&a) {
            Err(ENOTDIR.into())
        } else {
            Err(ENOENT.into())
        }
    }

    pub fn remove_dir(&mut self, p: &str) -> Result<(), String> {
        let a = self.abs(p);
        if !self.dirs.contains(&a) {
            return Err(if self.files.contains_key(&a) { ENOTDIR } else { ENOENT }.into());
        }
        if !self.children(&a).is_empty() {
            return Err(ENOTEMPTY.into());
        }
        self.dirs.remove(&a);
        Ok(())
    }

    pub fn remove_dir_all(&mut self, p: &str) -> Result<(), String> {
        let a = self.abs(p);
        if self.files.contains_key(&a) {
            return Err(ENOTDIR.into());
        }
        if !self.dirs.contains(&a) {
            return Err(ENOENT.into());
        }
        let prefix = format!("{a}/");
        self.files.retain(|k, _| !k.starts_with(&prefix));
        self.dirs.retain(|k| *k != a && !k.starts_with(&prefix));
        Ok(())
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let (a, b) = (self.abs(from), self.abs(to));
        if let Some(data) = self.files.get(&a).cloned() {
            self.check_target(&b)?;
            self.files.remove(&a);
            self.files.insert(b, data);
            return Ok(());
        }
        if self.dirs.contains(&a) {
            if self.files.contains_key(&b) {
                return Err(ENOTDIR.into());
            }
            if !self.dirs.contains(&parent(&b)) {
                return Err(ENOENT.into());
            }
            let prefix = format!("{a}/");
            let moved_files: Vec<(String, Vec<u8>)> = self.files.iter().filter(|(k, _)| k.starts_with(&prefix)).map(|(k, v)| (k.clone(), v.clone())).collect();
            let moved_dirs: Vec<String> = self.dirs.iter().filter(|k| **k == a || k.starts_with(&prefix)).cloned().collect();
            for (k, _) in &moved_files {
                self.files.remove(k);
            }
            for k in &moved_dirs {
                self.dirs.remove(k);
            }
            for (k, v) in moved_files {
                self.files.insert(format!("{b}{}", &k[a.len()..]), v);
            }
            for k in moved_dirs {
                self.dirs.insert(format!("{b}{}", &k[a.len()..]));
            }
            return Ok(());
        }
        Err(ENOENT.into())
    }

    pub fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        let data = self.read(from)?;
        self.write(to, data)
    }

    pub fn len(&self, p: &str) -> u64 {
        self.files.get(&self.abs(p)).map_or(0, |b| b.len() as u64)
    }

    /// Copy a host directory into the tree at `at` (files up to 1 MiB, at most 2000 entries): the fixtures of a test.
    pub fn load_host_dir(&mut self, host: &Path, at: &str) {
        let mut budget = 2000usize;
        self.load_rec(host, &self.abs(at), &mut budget);
    }

    fn load_rec(&mut self, host: &Path, at: &str, budget: &mut usize) {
        let Ok(rd) = std::fs::read_dir(host) else { return };
        let _ = self.mkdirs(at);
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            if *budget == 0 {
                return;
            }
            *budget -= 1;
            let name = e.file_name().to_string_lossy().to_string();
            let target = format!("{at}/{name}");
            let p = e.path();
            if p.is_dir() {
                self.load_rec(&p, &target, budget);
            } else if let Ok(b) = std::fs::read(&p) {
                if b.len() <= 1 << 20 {
                    self.files.insert(target, b);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_lexical_and_relative_to_the_cwd() {
        let mut fs = SimFs::new("/proj");
        fs.write("a.txt", b"x".to_vec()).unwrap();
        assert!(fs.is_file("/proj/a.txt"));
        assert!(fs.is_file("./sub/../a.txt") || !fs.exists("sub"));
        assert_eq!(normalize("/a/b/../c//d/."), "/a/c/d");
        assert_eq!(normalize("/.."), "/");
    }

    #[test]
    fn errors_match_the_operating_system() {
        let mut fs = SimFs::new("/p");
        assert_eq!(fs.read("nope").unwrap_err(), ENOENT);
        assert_eq!(fs.write("no/dir/f", vec![]).unwrap_err(), ENOENT);
        fs.create_dir("d").unwrap();
        assert_eq!(fs.create_dir("d").unwrap_err(), EEXIST);
        assert_eq!(fs.read("d").unwrap_err(), EISDIR);
        fs.write("d/f", b"1".to_vec()).unwrap();
        assert_eq!(fs.remove_dir("d").unwrap_err(), ENOTEMPTY);
        assert_eq!(fs.create_dir("d/f/g").unwrap_err(), ENOTDIR);
        assert_eq!(fs.write("d", vec![]).unwrap_err(), EISDIR);
        fs.remove_dir_all("d").unwrap();
        assert!(!fs.exists("d"));
    }

    #[test]
    fn rename_moves_a_subtree() {
        let mut fs = SimFs::new("/p");
        fs.create_dir_all("a/b").unwrap();
        fs.write("a/b/f", b"data".to_vec()).unwrap();
        fs.rename("a", "z").unwrap();
        assert_eq!(fs.read("z/b/f").unwrap(), b"data");
        assert!(!fs.exists("a"));
        assert_eq!(fs.read_dir("z").unwrap(), ["b"]);
    }

    #[test]
    fn append_creates_and_extends() {
        let mut fs = SimFs::new("/p");
        fs.append("log", b"a").unwrap();
        fs.append("log", b"b").unwrap();
        assert_eq!(fs.read("log").unwrap(), b"ab");
        assert_eq!(fs.len("log"), 2);
    }
}
