// Input handling for Windjammer games
// Provides a simple, high-level API for keyboard and mouse input

use std::collections::HashSet;
use winit::keyboard::{KeyCode, PhysicalKey};

/// Input state manager
///
/// Tracks which keys are currently pressed and provides a simple API
/// for querying input state in game code.
pub struct Input {
    keys_pressed: HashSet<Key>,
    keys_just_pressed: HashSet<Key>,
    keys_just_released: HashSet<Key>,

    // Mouse state
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_buttons_just_pressed: HashSet<MouseButton>,
    mouse_buttons_just_released: HashSet<MouseButton>,
    mouse_position: (f64, f64),
    mouse_delta: (f64, f64),
    last_mouse_position: Option<(f64, f64)>,
}

/// Key enum representing common keyboard keys
///
/// Maps to physical keyboard keys (QWERTY layout)
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

/// Mouse button enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl Input {
    /// Create a new Input state manager
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
            last_mouse_position: None,
        }
    }

    // ========================================
    // PRIMARY API: Natural, ergonomic methods
    // ========================================

    /// Check if a key is currently held down
    ///
    /// Use this for continuous actions like movement.
    /// Returns true every frame while the key is held.
    ///
    /// Example: `if input.held(Key::W) { player.move_forward() }`
    pub fn held(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key was pressed this frame
    ///
    /// Use this for one-shot actions like jumping or shooting.
    /// Returns true only on the frame when the key goes from up to down.
    ///
    /// Example: `if input.pressed(Key::Space) { player.jump() }`
    pub fn pressed(&self, key: Key) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    /// Check if a key was released this frame
    ///
    /// Use this for actions that trigger on release.
    /// Returns true only on the frame when the key goes from down to up.
    ///
    /// Example: `if input.released(Key::Space) { player.charge_release() }`
    pub fn released(&self, key: Key) -> bool {
        self.keys_just_released.contains(&key)
    }

    // ========================================
    // CONVENIENCE API: Common patterns
    // ========================================

    /// Check if ANY of the given keys is held
    ///
    /// Example: `if input.any_held(&[Key::W, Key::Up]) { player.move_forward() }`
    pub fn any_held(&self, keys: &[Key]) -> bool {
        keys.iter().any(|k| self.held(*k))
    }

    /// Check if ANY of the given keys was pressed
    ///
    /// Example: `if input.any_pressed(&[Key::Space, Key::Enter]) { confirm_action() }`
    pub fn any_pressed(&self, keys: &[Key]) -> bool {
        keys.iter().any(|k| self.pressed(*k))
    }

    /// Check if ALL of the given keys are held (for combos)
    ///
    /// Example: `if input.all_held(&[Key::Control, Key::S]) { save_game() }`
    pub fn all_held(&self, keys: &[Key]) -> bool {
        keys.iter().all(|k| self.held(*k))
    }

    // ========================================
    // MOUSE API: Mouse button and position
    // ========================================

    /// Check if a mouse button is currently held down
    ///
    /// Example: `if input.mouse_held(MouseButton::Left) { drag_object() }`
    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    /// Check if a mouse button was pressed this frame
    ///
    /// Example: `if input.mouse_pressed(MouseButton::Left) { shoot() }`
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    /// Check if a mouse button was released this frame
    ///
    /// Example: `if input.mouse_released(MouseButton::Left) { release_object() }`
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    /// Get the current mouse position (in pixels)
    ///
    /// Returns (x, y) where (0, 0) is top-left corner
    pub fn mouse_position(&self) -> (f64, f64) {
        self.mouse_position
    }

    /// Get the mouse movement delta since last frame
    ///
    /// Returns (dx, dy) in pixels. Useful for camera controls.
    ///
    /// Example: `let (dx, dy) = input.mouse_delta(); camera.rotate(dx, dy);`
    pub fn mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    /// Get the horizontal mouse movement delta (X axis)
    ///
    /// Returns dx in pixels. Positive = right, negative = left.
    ///
    /// Example: `let dx = input.mouse_delta_x(); camera.rotate_yaw(dx);`
    pub fn mouse_delta_x(&self) -> f64 {
        self.mouse_delta.0
    }

    /// Get the vertical mouse movement delta (Y axis)
    ///
    /// Returns dy in pixels. Positive = down, negative = up.
    ///
    /// Example: `let dy = input.mouse_delta_y(); camera.rotate_pitch(dy);`
    pub fn mouse_delta_y(&self) -> f64 {
        self.mouse_delta.1
    }

    // ========================================
    // LEGACY API: For compatibility
    // ========================================

    /// Alias for `held()` - check if a key is currently pressed
    #[deprecated(note = "Use `held()` instead for better readability")]
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.held(key)
    }

    /// Alias for `pressed()` - check if a key was just pressed
    #[deprecated(note = "Use `pressed()` instead for better readability")]
    pub fn is_key_just_pressed(&self, key: Key) -> bool {
        self.pressed(key)
    }

    /// Alias for `released()` - check if a key was just released
    #[deprecated(note = "Use `released()` instead for better readability")]
    pub fn is_key_just_released(&self, key: Key) -> bool {
        self.released(key)
    }

    // ========================================
    // TESTING API: For automated tests
    // ========================================

    /// Simulate a key press (for testing)
    ///
    /// This marks the key as pressed this frame and held.
    /// Use in tests to simulate user input.
    ///
    /// Example: `input.simulate_key_press(Key::W)`
    pub fn simulate_key_press(&mut self, key: Key) {
        if !self.keys_pressed.contains(&key) {
            self.keys_just_pressed.insert(key);
        }
        self.keys_pressed.insert(key);
    }

    /// Simulate a key release (for testing)
    ///
    /// This marks the key as released this frame.
    ///
    /// Example: `input.simulate_key_release(Key::W)`
    pub fn simulate_key_release(&mut self, key: Key) {
        self.keys_pressed.remove(&key);
        self.keys_just_released.insert(key);
    }

    /// Simulate a mouse button press (for testing)
    ///
    /// Example: `input.simulate_mouse_press(MouseButton::Left)`
    pub fn simulate_mouse_press(&mut self, button: MouseButton) {
        if !self.mouse_buttons_pressed.contains(&button) {
            self.mouse_buttons_just_pressed.insert(button);
        }
        self.mouse_buttons_pressed.insert(button);
    }

    /// Simulate a mouse button release (for testing)
    ///
    /// Example: `input.simulate_mouse_release(MouseButton::Left)`
    pub fn simulate_mouse_release(&mut self, button: MouseButton) {
        self.mouse_buttons_pressed.remove(&button);
        self.mouse_buttons_just_released.insert(button);
    }

    /// Simulate mouse movement (for testing)
    ///
    /// Sets the mouse position and calculates delta.
    ///
    /// Example: `input.simulate_mouse_move(100.0, 200.0)`
    pub fn simulate_mouse_move(&mut self, x: f64, y: f64) {
        if let Some(last_pos) = self.last_mouse_position {
            self.mouse_delta = (x - last_pos.0, y - last_pos.1);
        } else {
            self.mouse_delta = (0.0, 0.0);
        }
        self.mouse_position = (x, y);
        self.last_mouse_position = Some((x, y));
    }

    /// Simulate mouse delta movement (for testing camera controls)
    ///
    /// Directly sets the mouse delta without changing position.
    /// Useful for testing first-person camera controls.
    ///
    /// Example: `input.simulate_mouse_delta(10.0, -5.0)`
    pub fn simulate_mouse_delta(&mut self, dx: f64, dy: f64) {
        self.mouse_delta = (dx, dy);
    }

    // ========================================
    // INTERNAL API: Hidden from Windjammer users
    // ========================================

    /// Internal: Update input state from a winit keyboard event
    ///
    /// This method is used by the generated game loop code and should not be called directly.
    /// It's hidden from the public API to maintain zero-crate-leakage philosophy.
    #[doc(hidden)]
    pub fn update_from_winit(&mut self, event: &winit::event::KeyEvent) {
        if let Some(key) = Self::map_keycode(event.physical_key) {
            if event.state.is_pressed() {
                if !self.keys_pressed.contains(&key) {
                    self.keys_just_pressed.insert(key);
                }
                self.keys_pressed.insert(key);
            } else {
                self.keys_pressed.remove(&key);
                self.keys_just_released.insert(key);
            }
        }
    }

    /// Internal: Update mouse button state from winit event
    #[doc(hidden)]
    pub fn update_mouse_button_from_winit(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        if let Some(mb) = Self::map_mouse_button(button) {
            if state.is_pressed() {
                if !self.mouse_buttons_pressed.contains(&mb) {
                    self.mouse_buttons_just_pressed.insert(mb);
                }
                self.mouse_buttons_pressed.insert(mb);
            } else {
                self.mouse_buttons_pressed.remove(&mb);
                self.mouse_buttons_just_released.insert(mb);
            }
        }
    }

    /// Internal: Update mouse position from winit event
    #[doc(hidden)]
    pub fn update_mouse_position_from_winit(&mut self, x: f64, y: f64) {
        if let Some(last_pos) = self.last_mouse_position {
            self.mouse_delta = (x - last_pos.0, y - last_pos.1);
        } else {
            self.mouse_delta = (0.0, 0.0);
        }
        self.mouse_position = (x, y);
        self.last_mouse_position = Some((x, y));
    }

    /// Clear "just pressed" and "just released" states
    /// Should be called at the end of each frame
    pub fn clear_frame_state(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
        self.mouse_buttons_just_released.clear();
        self.mouse_delta = (0.0, 0.0);
    }

    /// Map winit KeyCode to our Key enum
    fn map_keycode(physical_key: PhysicalKey) -> Option<Key> {
        match physical_key {
            PhysicalKey::Code(code) => match code {
                // Letters
                KeyCode::KeyA => Some(Key::A),
                KeyCode::KeyB => Some(Key::B),
                KeyCode::KeyC => Some(Key::C),
                KeyCode::KeyD => Some(Key::D),
                KeyCode::KeyE => Some(Key::E),
                KeyCode::KeyF => Some(Key::F),
                KeyCode::KeyG => Some(Key::G),
                KeyCode::KeyH => Some(Key::H),
                KeyCode::KeyI => Some(Key::I),
                KeyCode::KeyJ => Some(Key::J),
                KeyCode::KeyK => Some(Key::K),
                KeyCode::KeyL => Some(Key::L),
                KeyCode::KeyM => Some(Key::M),
                KeyCode::KeyN => Some(Key::N),
                KeyCode::KeyO => Some(Key::O),
                KeyCode::KeyP => Some(Key::P),
                KeyCode::KeyQ => Some(Key::Q),
                KeyCode::KeyR => Some(Key::R),
                KeyCode::KeyS => Some(Key::S),
                KeyCode::KeyT => Some(Key::T),
                KeyCode::KeyU => Some(Key::U),
                KeyCode::KeyV => Some(Key::V),
                KeyCode::KeyW => Some(Key::W),
                KeyCode::KeyX => Some(Key::X),
                KeyCode::KeyY => Some(Key::Y),
                KeyCode::KeyZ => Some(Key::Z),

                // Numbers
                KeyCode::Digit0 => Some(Key::Num0),
                KeyCode::Digit1 => Some(Key::Num1),
                KeyCode::Digit2 => Some(Key::Num2),
                KeyCode::Digit3 => Some(Key::Num3),
                KeyCode::Digit4 => Some(Key::Num4),
                KeyCode::Digit5 => Some(Key::Num5),
                KeyCode::Digit6 => Some(Key::Num6),
                KeyCode::Digit7 => Some(Key::Num7),
                KeyCode::Digit8 => Some(Key::Num8),
                KeyCode::Digit9 => Some(Key::Num9),

                // Arrow keys
                KeyCode::ArrowUp => Some(Key::Up),
                KeyCode::ArrowDown => Some(Key::Down),
                KeyCode::ArrowLeft => Some(Key::Left),
                KeyCode::ArrowRight => Some(Key::Right),

                // Special keys
                KeyCode::Space => Some(Key::Space),
                KeyCode::Enter => Some(Key::Enter),
                KeyCode::Escape => Some(Key::Escape),
                KeyCode::Tab => Some(Key::Tab),
                KeyCode::Backspace => Some(Key::Backspace),
                KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Key::Shift),
                KeyCode::ControlLeft | KeyCode::ControlRight => Some(Key::Control),
                KeyCode::AltLeft | KeyCode::AltRight => Some(Key::Alt),

                // Function keys
                KeyCode::F1 => Some(Key::F1),
                KeyCode::F2 => Some(Key::F2),
                KeyCode::F3 => Some(Key::F3),
                KeyCode::F4 => Some(Key::F4),
                KeyCode::F5 => Some(Key::F5),
                KeyCode::F6 => Some(Key::F6),
                KeyCode::F7 => Some(Key::F7),
                KeyCode::F8 => Some(Key::F8),
                KeyCode::F9 => Some(Key::F9),
                KeyCode::F10 => Some(Key::F10),
                KeyCode::F11 => Some(Key::F11),
                KeyCode::F12 => Some(Key::F12),

                _ => None,
            },
            _ => None,
        }
    }

    /// Map winit MouseButton to our MouseButton enum
    fn map_mouse_button(button: winit::event::MouseButton) -> Option<MouseButton> {
        match button {
            winit::event::MouseButton::Left => Some(MouseButton::Left),
            winit::event::MouseButton::Right => Some(MouseButton::Right),
            winit::event::MouseButton::Middle => Some(MouseButton::Middle),
            _ => None,
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
