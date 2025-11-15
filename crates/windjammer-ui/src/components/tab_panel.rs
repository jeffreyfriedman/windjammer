use crate::prelude::ToVNode;
use crate::simple_vnode::{VAttr, VNode};
use std::cell::RefCell;
use std::rc::Rc;

/// TabPanel component for tabbed interfaces
pub struct TabPanel {
    tabs: Vec<Tab>,
    active_tab: usize,
}

pub struct Tab {
    pub label: String,
    pub content: VNode,
    pub closeable: bool,
    pub on_close: Option<Rc<RefCell<dyn FnMut()>>>,
}

impl TabPanel {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
        }
    }

    pub fn tabs(mut self, tabs: Vec<Tab>) -> Self {
        self.tabs = tabs;
        self
    }

    pub fn active_tab(mut self, index: usize) -> Self {
        self.active_tab = index;
        self
    }

    pub fn render(&self) -> VNode {
        let mut tab_buttons = Vec::new();
        for (i, tab) in self.tabs.iter().enumerate() {
            let is_active = i == self.active_tab;
            let class = if is_active {
                "tab-button active"
            } else {
                "tab-button"
            };

            tab_buttons.push(VNode::Element {
                tag: "button".to_string(),
                attrs: vec![("class".to_string(), VAttr::Static(class.to_string()))],
                children: vec![VNode::Text(tab.label.clone())],
            });
        }

        let tab_bar = VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static("tab-bar".to_string()))],
            children: tab_buttons,
        };

        let content = if let Some(tab) = self.tabs.get(self.active_tab) {
            tab.content.clone()
        } else {
            VNode::Text("".to_string())
        };

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static("tab-panel".to_string()))],
            children: vec![tab_bar, content],
        }
    }
}

impl Tab {
    pub fn new(label: impl Into<String>, content: VNode) -> Self {
        Self {
            label: label.into(),
            content,
            closeable: false,
            on_close: None,
        }
    }

    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: FnMut() + 'static,
    {
        self.on_close = Some(Rc::new(RefCell::new(handler)));
        self
    }
}

impl ToVNode for TabPanel {
    fn to_vnode(self) -> VNode {
        self.render()
    }
}
