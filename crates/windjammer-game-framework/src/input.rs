//! Input handling (keyboard, mouse, gamepad)

use std::collections::HashSet;

/// Input state
pub struct Input {
    keys_pressed: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_just_pressed: HashSet<MouseButton>,
    mouse_buttons_just_released: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    mouse_delta: (f32, f32),
    mouse_wheel_delta: f32,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_buttons_just_pressed: HashSet::new(),
            mouse_buttons_just_released: HashSet::new(),
            mouse_position: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            mouse_wheel_delta: 0.0,
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

    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    pub fn is_mouse_button_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    pub fn mouse_x(&self) -> f32 {
        self.mouse_position.0
    }

    pub fn mouse_y(&self) -> f32 {
        self.mouse_position.1
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    pub fn mouse_wheel_delta(&self) -> f32 {
        self.mouse_wheel_delta
    }

    pub fn clear_frame_state(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
        self.mouse_buttons_just_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_wheel_delta = 0.0;
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
        if !self.mouse_buttons_pressed.contains(&button) {
            self.mouse_buttons_just_pressed.insert(button);
        }
        self.mouse_buttons_pressed.insert(button);
    }

    /// Release a mouse button
    pub fn release_mouse_button(&mut self, button: MouseButton) {
        if self.mouse_buttons_pressed.contains(&button) {
            self.mouse_buttons_just_released.insert(button);
        }
        self.mouse_buttons_pressed.remove(&button);
    }

    /// Set mouse position
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        let old_pos = self.mouse_position;
        self.mouse_position = (x, y);
        self.mouse_delta = (x - old_pos.0, y - old_pos.1);
    }

    /// Set mouse delta (for manual control)
    pub fn set_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta = (dx, dy);
    }

    /// Set mouse wheel delta
    pub fn set_mouse_wheel_delta(&mut self, delta: f32) {
        self.mouse_wheel_delta = delta;
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
        assert_eq!(input.mouse_position(), (0.0, 0.0));
    }

    #[test]
    fn test_key_press() {
        let mut input = Input::new();

        input.press_key(KeyCode::W);
        assert!(input.is_key_pressed(KeyCode::W));
        assert!(input.is_key_just_pressed(KeyCode::W));

        // After clearing frame state, just_pressed should be false
        input.clear_frame_state();
        assert!(input.is_key_pressed(KeyCode::W));
        assert!(!input.is_key_just_pressed(KeyCode::W));
    }

    #[test]
    fn test_key_release() {
        let mut input = Input::new();

        input.press_key(KeyCode::Space);
        input.clear_frame_state();

        input.release_key(KeyCode::Space);
        assert!(!input.is_key_pressed(KeyCode::Space));
        assert!(input.is_key_just_released(KeyCode::Space));

        input.clear_frame_state();
        assert!(!input.is_key_just_released(KeyCode::Space));
    }

    #[test]
    fn test_mouse_button() {
        let mut input = Input::new();

        input.press_mouse_button(MouseButton::Left);
        assert!(input.is_mouse_button_pressed(MouseButton::Left));
        assert!(input.is_mouse_button_just_pressed(MouseButton::Left));

        input.clear_frame_state();
        assert!(input.is_mouse_button_pressed(MouseButton::Left));
        assert!(!input.is_mouse_button_just_pressed(MouseButton::Left));

        input.release_mouse_button(MouseButton::Left);
        assert!(!input.is_mouse_button_pressed(MouseButton::Left));
        assert!(input.is_mouse_button_just_released(MouseButton::Left));
    }

    #[test]
    fn test_mouse_position() {
        let mut input = Input::new();

        input.set_mouse_position(100.0, 200.0);
        assert_eq!(input.mouse_position(), (100.0, 200.0));
        assert_eq!(input.mouse_x(), 100.0);
        assert_eq!(input.mouse_y(), 200.0);
    }

    #[test]
    fn test_mouse_delta() {
        let mut input = Input::new();

        input.set_mouse_position(100.0, 100.0);
        input.clear_frame_state(); // Reset delta

        input.set_mouse_position(150.0, 120.0);
        assert_eq!(input.mouse_delta(), (50.0, 20.0));

        input.clear_frame_state();
        assert_eq!(input.mouse_delta(), (0.0, 0.0));
    }

    #[test]
    fn test_mouse_wheel() {
        let mut input = Input::new();

        input.set_mouse_wheel_delta(1.5);
        assert_eq!(input.mouse_wheel_delta(), 1.5);

        input.clear_frame_state();
        assert_eq!(input.mouse_wheel_delta(), 0.0);
    }

    #[test]
    fn test_multiple_keys() {
        let mut input = Input::new();

        input.press_key(KeyCode::W);
        input.press_key(KeyCode::A);
        input.press_key(KeyCode::S);

        assert!(input.is_key_pressed(KeyCode::W));
        assert!(input.is_key_pressed(KeyCode::A));
        assert!(input.is_key_pressed(KeyCode::S));
        assert!(!input.is_key_pressed(KeyCode::D));
    }
}
