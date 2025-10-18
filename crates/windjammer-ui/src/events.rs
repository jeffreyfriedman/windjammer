//! Cross-platform event system

pub mod dispatcher;

use std::fmt;
use std::sync::{Arc, Mutex};

pub use dispatcher::ComponentEventDispatcher;

/// Cross-platform event types
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Mouse click
    Click { x: f64, y: f64, button: MouseButton },
    /// Mouse move
    MouseMove { x: f64, y: f64 },
    /// Mouse down
    MouseDown { x: f64, y: f64, button: MouseButton },
    /// Mouse up
    MouseUp { x: f64, y: f64, button: MouseButton },
    /// Mouse enter
    MouseEnter,
    /// Mouse leave
    MouseLeave,
    /// Key press
    KeyPress { key: String, modifiers: Modifiers },
    /// Key down
    KeyDown { key: String, modifiers: Modifiers },
    /// Key up
    KeyUp { key: String, modifiers: Modifiers },
    /// Input change (for text inputs)
    Input { value: String },
    /// Change event
    Change { value: String },
    /// Focus
    Focus,
    /// Blur
    Blur,
    /// Form submit
    Submit,
    /// Touch event (mobile)
    Touch {
        x: f64,
        y: f64,
        touch_type: TouchType,
    },
    /// Scroll
    Scroll { x: f64, y: f64 },
    /// Resize
    Resize { width: u32, height: u32 },
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
            Event::MouseDown { x, y, button } => write!(f, "MouseDown({}, {}, {:?})", x, y, button),
            Event::MouseUp { x, y, button } => write!(f, "MouseUp({}, {}, {:?})", x, y, button),
            Event::MouseEnter => write!(f, "MouseEnter"),
            Event::MouseLeave => write!(f, "MouseLeave"),
            Event::KeyPress { key, .. } => write!(f, "KeyPress({})", key),
            Event::KeyDown { key, .. } => write!(f, "KeyDown({})", key),
            Event::KeyUp { key, .. } => write!(f, "KeyUp({})", key),
            Event::Input { value } => write!(f, "Input({})", value),
            Event::Change { value } => write!(f, "Change({})", value),
            Event::Focus => write!(f, "Focus"),
            Event::Blur => write!(f, "Blur"),
            Event::Submit => write!(f, "Submit"),
            Event::Touch { x, y, touch_type } => write!(f, "Touch({}, {}, {:?})", x, y, touch_type),
            Event::Scroll { x, y } => write!(f, "Scroll({}, {})", x, y),
            Event::Resize { width, height } => write!(f, "Resize({}, {})", width, height),
            Event::Custom { name, .. } => write!(f, "Custom({})", name),
        }
    }
}

/// Event context with propagation control
#[derive(Debug, Clone)]
pub struct EventContext {
    /// Target element ID
    pub target: String,
    /// Current element ID (for propagation)
    pub current_target: String,
    /// Event phase
    pub phase: EventPhase,
    /// Whether propagation is stopped
    stopped: Arc<Mutex<bool>>,
    /// Whether immediate propagation is stopped
    immediate_stopped: Arc<Mutex<bool>>,
    /// Whether default action is prevented
    default_prevented: Arc<Mutex<bool>>,
}

/// Event phase (capturing vs bubbling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    /// Capturing phase (from root to target)
    Capturing,
    /// At target
    AtTarget,
    /// Bubbling phase (from target to root)
    Bubbling,
}

impl EventContext {
    /// Create a new event context
    pub fn new(target: String) -> Self {
        Self {
            target: target.clone(),
            current_target: target,
            phase: EventPhase::AtTarget,
            stopped: Arc::new(Mutex::new(false)),
            immediate_stopped: Arc::new(Mutex::new(false)),
            default_prevented: Arc::new(Mutex::new(false)),
        }
    }

    /// Stop event propagation
    pub fn stop_propagation(&self) {
        *self.stopped.lock().unwrap() = true;
    }

    /// Stop immediate event propagation (prevents other listeners on same element)
    pub fn stop_immediate_propagation(&self) {
        *self.immediate_stopped.lock().unwrap() = true;
    }

    /// Prevent default action
    pub fn prevent_default(&self) {
        *self.default_prevented.lock().unwrap() = true;
    }

    /// Check if propagation is stopped
    pub fn is_propagation_stopped(&self) -> bool {
        *self.stopped.lock().unwrap()
    }

    /// Check if immediate propagation is stopped
    pub fn is_immediate_propagation_stopped(&self) -> bool {
        *self.immediate_stopped.lock().unwrap()
    }

    /// Check if default is prevented
    pub fn is_default_prevented(&self) -> bool {
        *self.default_prevented.lock().unwrap()
    }
}

/// Event listener with phase
pub struct EventListener {
    /// Event type (e.g., "click", "keydown")
    pub event_type: String,
    /// Handler function
    #[allow(clippy::type_complexity)]
    pub handler: Arc<dyn Fn(&Event, &EventContext) + Send + Sync>,
    /// Whether to capture
    pub capture: bool,
}

impl EventListener {
    /// Create a new event listener
    pub fn new<F>(event_type: String, handler: F, capture: bool) -> Self
    where
        F: Fn(&Event, &EventContext) + Send + Sync + 'static,
    {
        Self {
            event_type,
            handler: Arc::new(handler),
            capture,
        }
    }

    /// Invoke the handler
    pub fn invoke(&self, event: &Event, context: &EventContext) {
        (self.handler)(event, context);
    }
}

/// Event dispatcher for managing event listeners
pub struct EventDispatcher {
    /// Listeners by element ID
    listeners: Arc<Mutex<std::collections::HashMap<String, Vec<EventListener>>>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Add an event listener
    pub fn add_listener(&self, element_id: String, listener: EventListener) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.entry(element_id).or_default().push(listener);
    }

    /// Remove all listeners for an element
    pub fn remove_listeners(&self, element_id: &str) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.remove(element_id);
    }

    /// Dispatch an event with propagation
    pub fn dispatch(&self, event: &Event, target_id: String, path: Vec<String>) {
        let context = EventContext::new(target_id.clone());

        // Capturing phase (from root to target)
        let mut ctx = context.clone();
        ctx.phase = EventPhase::Capturing;
        for element_id in path.iter().rev() {
            if ctx.is_propagation_stopped() {
                break;
            }
            ctx.current_target = element_id.clone();
            self.invoke_listeners(element_id, event, &ctx, true);
        }

        // At target
        if !context.is_propagation_stopped() {
            let mut ctx = context.clone();
            ctx.phase = EventPhase::AtTarget;
            ctx.current_target = target_id.clone();
            self.invoke_listeners(&target_id, event, &ctx, false);
        }

        // Bubbling phase (from target to root)
        if !context.is_propagation_stopped() {
            let mut ctx = context.clone();
            ctx.phase = EventPhase::Bubbling;
            for element_id in path.iter() {
                if ctx.is_propagation_stopped() {
                    break;
                }
                ctx.current_target = element_id.clone();
                self.invoke_listeners(element_id, event, &ctx, false);
            }
        }
    }

    fn invoke_listeners(
        &self,
        element_id: &str,
        event: &Event,
        context: &EventContext,
        capture_phase: bool,
    ) {
        let listeners = self.listeners.lock().unwrap();
        if let Some(element_listeners) = listeners.get(element_id) {
            for listener in element_listeners {
                if context.is_immediate_propagation_stopped() {
                    break;
                }
                if listener.capture == capture_phase {
                    listener.invoke(event, context);
                }
            }
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_event_context_stop_propagation() {
        let ctx = EventContext::new("button".to_string());
        assert!(!ctx.is_propagation_stopped());

        ctx.stop_propagation();
        assert!(ctx.is_propagation_stopped());
    }

    #[test]
    fn test_event_context_prevent_default() {
        let ctx = EventContext::new("link".to_string());
        assert!(!ctx.is_default_prevented());

        ctx.prevent_default();
        assert!(ctx.is_default_prevented());
    }

    #[test]
    fn test_event_dispatcher_add_listener() {
        let dispatcher = EventDispatcher::new();
        let called = std::sync::Arc::new(std::sync::Mutex::new(0));
        let called_clone = called.clone();

        let listener = EventListener::new(
            "click".to_string(),
            move |_event, _ctx| {
                *called_clone.lock().unwrap() += 1;
            },
            false,
        );

        dispatcher.add_listener("button".to_string(), listener);

        let event = Event::Click {
            x: 10.0,
            y: 20.0,
            button: MouseButton::Left,
        };
        dispatcher.dispatch(&event, "button".to_string(), vec!["root".to_string()]);

        assert_eq!(*called.lock().unwrap(), 1);
    }

    #[test]
    fn test_event_propagation_bubbling() {
        let dispatcher = EventDispatcher::new();
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        // Add listener to parent
        let events_clone = events.clone();
        let parent_listener = EventListener::new(
            "click".to_string(),
            move |_event, ctx| {
                events_clone
                    .lock()
                    .unwrap()
                    .push(ctx.current_target.clone());
            },
            false,
        );
        dispatcher.add_listener("parent".to_string(), parent_listener);

        // Add listener to child
        let events_clone = events.clone();
        let child_listener = EventListener::new(
            "click".to_string(),
            move |_event, ctx| {
                events_clone
                    .lock()
                    .unwrap()
                    .push(ctx.current_target.clone());
            },
            false,
        );
        dispatcher.add_listener("child".to_string(), child_listener);

        let event = Event::Click {
            x: 10.0,
            y: 20.0,
            button: MouseButton::Left,
        };

        // Dispatch from child to parent
        dispatcher.dispatch(&event, "child".to_string(), vec!["parent".to_string()]);

        let recorded_events = events.lock().unwrap();
        assert_eq!(recorded_events.len(), 2);
        assert_eq!(recorded_events[0], "child"); // Target first
        assert_eq!(recorded_events[1], "parent"); // Then bubbles to parent
    }

    #[test]
    fn test_event_stop_propagation() {
        let dispatcher = EventDispatcher::new();
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        // Add listener to parent
        let events_clone = events.clone();
        let parent_listener = EventListener::new(
            "click".to_string(),
            move |_event, ctx| {
                events_clone
                    .lock()
                    .unwrap()
                    .push(ctx.current_target.clone());
            },
            false,
        );
        dispatcher.add_listener("parent".to_string(), parent_listener);

        // Add listener to child that stops propagation
        let events_clone = events.clone();
        let child_listener = EventListener::new(
            "click".to_string(),
            move |_event, ctx| {
                events_clone
                    .lock()
                    .unwrap()
                    .push(ctx.current_target.clone());
                ctx.stop_propagation();
            },
            false,
        );
        dispatcher.add_listener("child".to_string(), child_listener);

        let event = Event::Click {
            x: 10.0,
            y: 20.0,
            button: MouseButton::Left,
        };

        dispatcher.dispatch(&event, "child".to_string(), vec!["parent".to_string()]);

        let recorded_events = events.lock().unwrap();
        assert_eq!(recorded_events.len(), 1); // Only child, propagation stopped
        assert_eq!(recorded_events[0], "child");
    }
}
