//! File browser component

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub children: Vec<FileNode>,
}

impl FileNode {
    pub fn new(name: String, path: String, is_directory: bool) -> Self {
        Self {
            name,
            path,
            is_directory,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: FileNode) {
        self.children.push(child);
    }
}

pub struct FileBrowser {
    pub root: FileNode,
    pub current_file: Option<String>,
}

impl FileBrowser {
    pub fn new() -> Self {
        let mut root = FileNode::new("project".to_string(), "/".to_string(), true);

        // Add default files
        root.add_child(FileNode::new(
            "main.wj".to_string(),
            "/main.wj".to_string(),
            false,
        ));
        root.add_child(FileNode::new(
            "README.md".to_string(),
            "/README.md".to_string(),
            false,
        ));

        Self {
            root,
            current_file: Some("/main.wj".to_string()),
        }
    }

    pub fn get_current_file(&self) -> Option<&String> {
        self.current_file.as_ref()
    }

    pub fn set_current_file(&mut self, path: String) {
        self.current_file = Some(path);
    }
}

impl Default for FileBrowser {
    fn default() -> Self {
        Self::new()
    }
}
