



use smallvec::{SmallVec, smallvec};

use windjammer_runtime::fs;

use windjammer_runtime::path;

use windjammer_runtime::collections::HashSet;


struct GitignoreRules {
    patterns: Vec<String>,
}

impl GitignoreRules {
#[inline]
fn new() -> Self {
        GitignoreRules { patterns: vec![] }
}
fn load_from_directory(&self, mut dir: String) -> Result<Self, String> {
        let gitignore_path = path::join(&dir, &".gitignore");
        if !fs::exists(&gitignore_path) {
            return Ok(GitignoreRules::new());
        }
        let contents = fs::read_to_string(&gitignore_path)?;
        let mut patterns: SmallVec<[_; 4]> = smallvec![];
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("#") {
                continue;
            }
            self.patterns.push(trimmed.to_string());
        }
        Ok(GitignoreRules { patterns })
}
#[inline]
fn is_ignored(&self, mut path: &str) -> bool {
        let name = path::file_name(&path).unwrap_or(path);
        let path_str = path.clone();
        for pattern in self.patterns.iter() {
            if self.matches_pattern(name.clone(), pattern.clone()) || self.matches_pattern(path_str.clone(), pattern.clone()) {
                return true;
            }
        }
        false
}
fn matches_pattern(&self, mut name: String, mut pattern: &str) -> bool {
        if name == pattern {
            return true;
        }
        if pattern.ends_with("/") {
            let dir_pattern = pattern.trim_end_matches("/");
            if name == dir_pattern {
                return true;
            }
        }
        if pattern.contains("*") {
            return self.wildcard_match(name, pattern);
        }
        if pattern.starts_with("*.") {
            let ext = pattern.trim_start_matches("*.");
            if name.ends_with(&format!(".{}", ext)) {
                return true;
            }
        }
        if name.contains(pattern) {
            return true;
        }
        false
}
fn wildcard_match(&self, mut name: String, mut pattern: &str) -> bool {
        let parts: Vec<String> = pattern.split('*').collect();
        if parts.is_empty() {
            return false;
        }
        if !parts[0].is_empty() && !name.starts_with(parts[0]) {
            return false;
        }
        if parts.len() > 1 {
            let last = &parts[parts.len() - 1];
            if !last.is_empty() && !name.ends_with(last) {
                return false;
            }
        }
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i == 0 {
                pos = part.len();
                continue;
            }
            match &name[pos..name.len()].find(part) {
                Some(idx) => {
                    pos = pos + idx + part.len();
                },
                _ => {
                    return false;
                },
            }
        }
        true
}
}

struct GitignoreCache {
    cache: std::collections::HashMap<String, GitignoreRules>,
}

impl GitignoreCache {
#[inline]
fn new() -> Self {
        GitignoreRules { patterns: vec![] }
}
#[inline]
fn get_rules(&mut self, mut dir: &str) -> GitignoreRules {
        match self.cache.get(&dir) {
            Some(rules) => {
                return rules.clone();
            },
        }
        let rules = GitignoreRules::load_from_directory(dir.clone()).unwrap_or_else(move |_| GitignoreRules::new());
        self.cache.insert(dir, rules.clone());
        rules
}
}

