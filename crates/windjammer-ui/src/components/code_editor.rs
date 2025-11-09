//! Code Editor component for Windjammer Editor

use crate::prelude::*;
use crate::simple_vnode::{VAttr, VNode};

pub struct CodeEditor {
    pub content: Signal<String>,
    pub language: String,
    pub theme: String,
    pub read_only: bool,
    pub line_numbers: bool,
    pub on_change: Option<Box<dyn Fn(String) + 'static>>,
}

impl CodeEditor {
    pub fn new(content: Signal<String>) -> Self {
        Self {
            content,
            language: "windjammer".to_string(),
            theme: "vs-dark".to_string(),
            read_only: false,
            line_numbers: true,
            on_change: None,
        }
    }
    
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }
    
    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = theme.into();
        self
    }
    
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
    
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }
    
    pub fn on_change<F: Fn(String) + 'static>(mut self, handler: F) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
    
    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-code-editor".to_string()];
        classes.push(format!("wj-code-editor-{}", self.theme));
        
        if self.read_only {
            classes.push("wj-code-editor-readonly".to_string());
        }
        
        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![
                VAttr {
                    name: "class".to_string(),
                    value: classes.join(" "),
                },
                VAttr {
                    name: "data-language".to_string(),
                    value: self.language.clone(),
                },
            ],
            children: vec![
                VNode::Element {
                    tag: "textarea".to_string(),
                    attrs: vec![
                        VAttr {
                            name: "class".to_string(),
                            value: "wj-code-editor-textarea".to_string(),
                        },
                        VAttr {
                            name: "spellcheck".to_string(),
                            value: "false".to_string(),
                        },
                        VAttr {
                            name: "readonly".to_string(),
                            value: if self.read_only { "true" } else { "false" }.to_string(),
                        },
                    ],
                    children: vec![VNode::Text(self.content.get())],
                }
            ],
        }
    }
}

