//! Input handling (keyboard, mouse, gamepad)

use std::collections::HashSet;

/// Input state
pub struct Input {
    keys_pressed: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_position: (f32, f32),
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_position: (0.0, 0.0),
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn clear_frame_state(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
    }

    /// Press a key (for testing or manual input)
    pub fn press_key(&mut self, key: KeyCode) {
        if !self.keys_pressed.contains(&key) {
            self.keys_just_pressed.insert(key);
        }
        self.keys_pressed.insert(key);
    }

    /// Release a key (for testing or manual input)
    pub fn release_key(&mut self, key: KeyCode) {
        if self.keys_pressed.contains(&key) {
            self.keys_just_released.insert(key);
        }
        self.keys_pressed.remove(&key);
    }

    /// Press a mouse button
    pub fn press_mouse_button(&mut self, button: MouseButton) {
        self.mouse_buttons_pressed.insert(button);
    }

    /// Release a mouse button
    pub fn release_mouse_button(&mut self, button: MouseButton) {
        self.mouse_buttons_pressed.remove(&button);
    }

    /// Set mouse position
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

/// Keyboard key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Arrow keys
    Up,
    Down,
    Left,
    Right,

    // Special keys
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Shift,
    Control,
    Alt,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_creation() {
        let input = Input::new();
        assert!(!input.is_key_pressed(KeyCode::Space));
    }
}

    // Special keys
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Shift,
    Control,
    Alt,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_creation() {
        let input = Input::new();
        assert!(!input.is_key_pressed(KeyCode::Space));
    }
}
