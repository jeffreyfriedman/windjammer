//! Cross-platform event system

use std::fmt;

/// Cross-platform event types
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Mouse click
    Click { x: f64, y: f64, button: MouseButton },
    /// Mouse move
    MouseMove { x: f64, y: f64 },
    /// Key press
    KeyPress { key: String, modifiers: Modifiers },
    /// Input change (for text inputs)
    Input { value: String },
    /// Form submit
    Submit,
    /// Touch event (mobile)
    Touch {
        x: f64,
        y: f64,
        touch_type: TouchType,
    },
    /// Custom event
    Custom { name: String, data: String },
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Touch event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchType {
    Start,
    Move,
    End,
    Cancel,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Event handler trait
pub trait EventHandler: Send + Sync {
    /// Handle an event
    fn handle(&self, event: Event);
}

/// Function-based event handler
pub struct FnHandler<F>
where
    F: Fn(Event) + Send + Sync,
{
    handler: F,
}

impl<F> FnHandler<F>
where
    F: Fn(Event) + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> EventHandler for FnHandler<F>
where
    F: Fn(Event) + Send + Sync,
{
    fn handle(&self, event: Event) {
        (self.handler)(event);
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Click { x, y, button } => write!(f, "Click({}, {}, {:?})", x, y, button),
            Event::MouseMove { x, y } => write!(f, "MouseMove({}, {})", x, y),
            Event::KeyPress { key, .. } => write!(f, "KeyPress({})", key),
            Event::Input { value } => write!(f, "Input({})", value),
            Event::Submit => write!(f, "Submit"),
            Event::Touch { x, y, touch_type } => write!(f, "Touch({}, {}, {:?})", x, y, touch_type),
            Event::Custom { name, .. } => write!(f, "Custom({})", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_display() {
        let event = Event::Click {
            x: 10.0,
            y: 20.0,
            button: MouseButton::Left,
        };
        assert!(event.to_string().contains("Click"));
    }

    #[test]
    fn test_modifiers_default() {
        let mods = Modifiers::default();
        assert!(!mods.ctrl);
        assert!(!mods.alt);
        assert!(!mods.shift);
        assert!(!mods.meta);
    }

    #[test]
    fn test_fn_handler() {
        let called = std::sync::Arc::new(std::sync::Mutex::new(false));
        let called_clone = called.clone();

        let handler = FnHandler::new(move |_event| {
            *called_clone.lock().unwrap() = true;
        });

        handler.handle(Event::Submit);
        assert!(*called.lock().unwrap());
    }
}
