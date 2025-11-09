//! Code editor component

/// Code editor state
pub struct Editor {
    pub content: String,
    pub cursor_position: usize,
    pub language: String,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            language: "windjammer".to_string(),
        }
    }
    
    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }
    
    pub fn get_content(&self) -> &str {
        &self.content
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

