use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};
use std::cell::RefCell;
use std::rc::Rc;

/// Advanced CodeEditor with syntax highlighting support
pub struct AdvancedCodeEditor {
    content: Rc<RefCell<String>>,
    language: String,
    line_numbers: bool,
    read_only: bool,
    on_change: Option<Rc<RefCell<dyn FnMut(String)>>>,
}

impl AdvancedCodeEditor {
    pub fn new(content: Rc<RefCell<String>>) -> Self {
        Self {
            content,
            language: "windjammer".to_string(),
            line_numbers: true,
            read_only: false,
            on_change: None,
        }
    }

    pub fn language(mut self, lang: impl Into<String>) -> Self {
        self.language = lang.into();
        self
    }

    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn on_change<F>(mut self, handler: F) -> Self
    where
        F: FnMut(String) + 'static,
    {
        self.on_change = Some(Rc::new(RefCell::new(handler)));
        self
    }

    pub fn render(&self) -> VNode {
        let content = self.content.borrow().clone();
        let lines: Vec<&str> = content.lines().collect();

        let mut editor_children = Vec::new();

        // Line numbers column
        if self.line_numbers {
            let mut line_num_children = Vec::new();
            for i in 1..=lines.len() {
                line_num_children.push(VNode::Text(format!("{}\n", i)));
            }

            editor_children.push(VNode::Element {
                tag: "div".to_string(),
                attrs: vec![(
                    "class".to_string(),
                    VAttr::Static("line-numbers".to_string()),
                )],
                children: line_num_children,
            });
        }

        // Code content with syntax highlighting
        let highlighted = self.highlight_syntax(&content);

        editor_children.push(VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static("code-content".to_string()),
            )],
            children: vec![highlighted],
        });

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![(
                "class".to_string(),
                VAttr::Static(format!("advanced-code-editor language-{}", self.language)),
            )],
            children: editor_children,
        }
    }

    fn highlight_syntax(&self, code: &str) -> VNode {
        // Basic syntax highlighting for Windjammer
        let mut spans = Vec::new();

        for line in code.lines() {
            let highlighted_line = self.highlight_line(line);
            spans.push(highlighted_line);
            spans.push(VNode::Text("\n".to_string()));
        }

        VNode::Element {
            tag: "pre".to_string(),
            attrs: vec![],
            children: spans,
        }
    }

    fn highlight_line(&self, line: &str) -> VNode {
        // Simple keyword highlighting
        let keywords = [
            "fn", "let", "mut", "use", "pub", "struct", "enum", "impl", "if", "else", "match",
            "return",
        ];
        let types = ["int", "float", "string", "bool", "Vec"];

        let mut result = Vec::new();
        let mut current = String::new();

        for word in line.split_whitespace() {
            if keywords.contains(&word) {
                if !current.is_empty() {
                    result.push(VNode::Text(current.clone()));
                    current.clear();
                }
                result.push(VNode::Element {
                    tag: "span".to_string(),
                    attrs: vec![("class".to_string(), VAttr::Static("keyword".to_string()))],
                    children: vec![VNode::Text(word.to_string())],
                });
                result.push(VNode::Text(" ".to_string()));
            } else if types.contains(&word) {
                if !current.is_empty() {
                    result.push(VNode::Text(current.clone()));
                    current.clear();
                }
                result.push(VNode::Element {
                    tag: "span".to_string(),
                    attrs: vec![("class".to_string(), VAttr::Static("type".to_string()))],
                    children: vec![VNode::Text(word.to_string())],
                });
                result.push(VNode::Text(" ".to_string()));
            } else if word.starts_with("//") {
                if !current.is_empty() {
                    result.push(VNode::Text(current.clone()));
                    current.clear();
                }
                result.push(VNode::Element {
                    tag: "span".to_string(),
                    attrs: vec![("class".to_string(), VAttr::Static("comment".to_string()))],
                    children: vec![VNode::Text(word.to_string())],
                });
                result.push(VNode::Text(" ".to_string()));
            } else if word.starts_with('"') || word.starts_with('\'') {
                if !current.is_empty() {
                    result.push(VNode::Text(current.clone()));
                    current.clear();
                }
                result.push(VNode::Element {
                    tag: "span".to_string(),
                    attrs: vec![("class".to_string(), VAttr::Static("string".to_string()))],
                    children: vec![VNode::Text(word.to_string())],
                });
                result.push(VNode::Text(" ".to_string()));
            } else {
                current.push_str(word);
                current.push(' ');
            }
        }

        if !current.is_empty() {
            result.push(VNode::Text(current));
        }

        VNode::Element {
            tag: "span".to_string(),
            attrs: vec![],
            children: result,
        }
    }
}

impl ToVNode for AdvancedCodeEditor {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
