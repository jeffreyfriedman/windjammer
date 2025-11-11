//! Project management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub files: HashMap<String, String>,
    pub entry_point: String,
}

impl Project {
    pub fn new(name: String) -> Self {
        let mut files = HashMap::new();
        files.insert(
            "main.wj".to_string(),
            r#"fn main() {
    println("Hello, Windjammer!")
}
"#
            .to_string(),
        );

        Self {
            name,
            files,
            entry_point: "main.wj".to_string(),
        }
    }

    pub fn add_file(&mut self, path: String, content: String) {
        self.files.insert(path, content);
    }

    pub fn get_file(&self, path: &str) -> Option<&String> {
        self.files.get(path)
    }

    pub fn update_file(&mut self, path: String, content: String) {
        self.files.insert(path, content);
    }

    pub fn remove_file(&mut self, path: &str) {
        self.files.remove(path);
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new("My Project".to_string())
    }
}
