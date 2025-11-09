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
}

/// Key enum representing common keyboard keys
/// 
/// Maps to physical keyboard keys (QWERTY layout)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // Numbers
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    
    // Arrow keys
    Up, Down, Left, Right,
    
    // Special keys
    Space, Enter, Escape, Tab, Backspace,
    Shift, Control, Alt,
    
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
}

impl Input {
    /// Create a new Input state manager
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
        }
    }

    /// Check if a key is currently pressed
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Check if a key was just pressed this frame
    pub fn is_key_just_pressed(&self, key: Key) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    /// Check if a key was just released this frame
    pub fn is_key_just_released(&self, key: Key) -> bool {
        self.keys_just_released.contains(&key)
    }

    /// Update input state from a winit keyboard event
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

    /// Clear "just pressed" and "just released" states
    /// Should be called at the end of each frame
    pub fn clear_frame_state(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
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
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
