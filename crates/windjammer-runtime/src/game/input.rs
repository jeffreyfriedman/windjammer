// Input handling for games
// Provides keyboard, mouse, and gamepad input

use std::collections::HashSet;

/// Key codes for keyboard input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
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
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

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
    Shift,
    Control,
    Alt,

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
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input state manager
pub struct Input {
    keys_down: HashSet<Key>,
    keys_pressed: HashSet<Key>,  // Just this frame
    keys_released: HashSet<Key>, // Just this frame

    mouse_buttons_down: HashSet<MouseButton>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_released: HashSet<MouseButton>,

    mouse_position: (f32, f32),
    mouse_delta: (f32, f32),
    mouse_wheel: f32,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Input {
    fn default() -> Self {
        Input {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_buttons_down: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_released: HashSet::new(),
            mouse_position: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            mouse_wheel: 0.0,
        }
    }
}

impl Input {
    /// Call at the start of each frame to update state
    pub fn update(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_buttons_pressed.clear();
        self.mouse_buttons_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_wheel = 0.0;
    }

    /// Check if a key is currently held down
    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    /// Check if a key was just pressed this frame
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key was just released this frame
    pub fn is_key_released(&self, key: Key) -> bool {
        self.keys_released.contains(&key)
    }

    /// Internal: Register a key press
    pub fn press_key(&mut self, key: Key) {
        if !self.keys_down.contains(&key) {
            self.keys_pressed.insert(key);
        }
        self.keys_down.insert(key);
    }

    /// Internal: Register a key release
    pub fn release_key(&mut self, key: Key) {
        self.keys_down.remove(&key);
        self.keys_released.insert(key);
    }

    /// Check if a mouse button is currently held down
    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Check if a mouse button was just pressed this frame
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    /// Check if a mouse button was just released this frame
    pub fn is_mouse_button_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_released.contains(&button)
    }

    /// Internal: Register a mouse button press
    pub fn press_mouse_button(&mut self, button: MouseButton) {
        if !self.mouse_buttons_down.contains(&button) {
            self.mouse_buttons_pressed.insert(button);
        }
        self.mouse_buttons_down.insert(button);
    }

    /// Internal: Register a mouse button release
    pub fn release_mouse_button(&mut self, button: MouseButton) {
        self.mouse_buttons_down.remove(&button);
        self.mouse_buttons_released.insert(button);
    }

    /// Get current mouse position
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Get mouse movement since last frame
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    /// Get mouse wheel scroll this frame
    pub fn mouse_wheel(&self) -> f32 {
        self.mouse_wheel
    }

    /// Internal: Update mouse position
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        let old_pos = self.mouse_position;
        self.mouse_position = (x, y);
        self.mouse_delta = (x - old_pos.0, y - old_pos.1);
    }

    /// Internal: Add mouse wheel movement
    pub fn add_mouse_wheel(&mut self, delta: f32) {
        self.mouse_wheel += delta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_press_release() {
        let mut input = Input::new();

        // Press a key
        input.press_key(Key::Space);
        assert!(input.is_key_down(Key::Space));
        assert!(input.is_key_pressed(Key::Space));

        // Next frame - still down but not pressed
        input.update();
        assert!(input.is_key_down(Key::Space));
        assert!(!input.is_key_pressed(Key::Space));

        // Release the key
        input.release_key(Key::Space);
        assert!(!input.is_key_down(Key::Space));
        assert!(input.is_key_released(Key::Space));

        // Next frame - neither down nor released
        input.update();
        assert!(!input.is_key_down(Key::Space));
        assert!(!input.is_key_released(Key::Space));
    }

    #[test]
    fn test_mouse_button() {
        let mut input = Input::new();

        input.press_mouse_button(MouseButton::Left);
        assert!(input.is_mouse_button_down(MouseButton::Left));
        assert!(input.is_mouse_button_pressed(MouseButton::Left));

        input.update();
        assert!(input.is_mouse_button_down(MouseButton::Left));
        assert!(!input.is_mouse_button_pressed(MouseButton::Left));
    }

    #[test]
    fn test_mouse_position() {
        let mut input = Input::new();

        input.set_mouse_position(100.0, 200.0);
        assert_eq!(input.mouse_position(), (100.0, 200.0));
        assert_eq!(input.mouse_delta(), (100.0, 200.0));

        input.update();
        input.set_mouse_position(150.0, 250.0);
        assert_eq!(input.mouse_delta(), (50.0, 50.0));
    }
}
