//! Alert component for messages
use crate::simple_vnode::{VAttr, VNode};

pub enum AlertVariant {
    Error,
    Warning,
    Info,
    Success,
}

pub struct Alert {
    pub message: String,
    pub variant: AlertVariant,
}

impl Alert {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: AlertVariant::Error,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: AlertVariant::Warning,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: AlertVariant::Info,
        }
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            variant: AlertVariant::Success,
        }
    }

    pub fn render(&self) -> VNode {
        let mut classes = vec!["wj-alert".to_string()];

        classes.push(
            match self.variant {
                AlertVariant::Error => "wj-alert-error",
                AlertVariant::Warning => "wj-alert-warning",
                AlertVariant::Info => "wj-alert-info",
                AlertVariant::Success => "wj-alert-success",
            }
            .to_string(),
        );

        let icon = match self.variant {
            AlertVariant::Error => "❌",
            AlertVariant::Warning => "⚠️",
            AlertVariant::Info => "ℹ️",
            AlertVariant::Success => "✅",
        };

        VNode::Element {
            tag: "div".to_string(),
            attrs: vec![("class".to_string(), VAttr::Static(classes.join(" ")))],
            children: vec![VNode::Text(format!("{} {}", icon, self.message))],
        }
    }
}
