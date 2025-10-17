//! Input handling for games

use std::collections::HashSet;

/// Keyboard keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Space,
    Enter,
    Escape,
    Left,
    Right,
    Up,
    Down,
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
}

/// Mouse buttons for games
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input state manager
pub struct Input {
    keys_pressed: HashSet<Key>,
    keys_just_pressed: HashSet<Key>,
    keys_just_released: HashSet<Key>,
    mouse_position: (f32, f32),
    mouse_buttons_pressed: HashSet<MouseButton>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_position: (0.0, 0.0),
            mouse_buttons_pressed: HashSet::new(),
        }
    }

    /// Check if a key is currently pressed
    pub fn key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key was just pressed this frame
    pub fn key_just_pressed(&self, key: Key) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    /// Check if a key was just released this frame
    pub fn key_just_released(&self, key: Key) -> bool {
        self.keys_just_released.contains(&key)
    }

    /// Get mouse position
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Check if a mouse button is pressed
    pub fn mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    /// Update input state (called internally each frame)
    pub fn update(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
    }

    /// Press a key (called by platform layer)
    pub fn press_key(&mut self, key: Key) {
        if !self.keys_pressed.contains(&key) {
            self.keys_just_pressed.insert(key);
        }
        self.keys_pressed.insert(key);
    }

    /// Release a key (called by platform layer)
    pub fn release_key(&mut self, key: Key) {
        if self.keys_pressed.contains(&key) {
            self.keys_just_released.insert(key);
        }
        self.keys_pressed.remove(&key);
    }

    /// Set mouse position (called by platform layer)
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_key_press() {
        let mut input = Input::new();
        input.press_key(Key::Space);

        assert!(input.key_pressed(Key::Space));
        assert!(input.key_just_pressed(Key::Space));

        input.update();
        assert!(input.key_pressed(Key::Space));
        assert!(!input.key_just_pressed(Key::Space));
    }

    #[test]
    fn test_input_key_release() {
        let mut input = Input::new();
        input.press_key(Key::Space);
        input.update();
        input.release_key(Key::Space);

        assert!(!input.key_pressed(Key::Space));
        assert!(input.key_just_released(Key::Space));
    }

    #[test]
    fn test_mouse_position() {
        let mut input = Input::new();
        input.set_mouse_position(100.0, 200.0);
        assert_eq!(input.mouse_position(), (100.0, 200.0));
    }
}
