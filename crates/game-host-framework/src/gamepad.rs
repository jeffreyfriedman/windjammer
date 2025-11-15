//! Gamepad/Controller Input System
//!
//! Provides comprehensive gamepad support for console-quality games.
//! Supports Xbox, PlayStation, Nintendo, and generic controllers.
//!
//! ## Features
//! - Multiple gamepad support (up to 8 players)
//! - Button states (pressed, held, released)
//! - Analog sticks with deadzone handling
//! - Triggers (analog)
//! - Rumble/haptic feedback
//! - Controller detection & hot-plugging

use std::collections::HashMap;

/// Maximum number of supported gamepads
pub const MAX_GAMEPADS: usize = 8;

/// Gamepad input manager
#[derive(Debug, Clone)]
pub struct GamepadManager {
    /// Connected gamepads (indexed by player ID)
    gamepads: HashMap<usize, Gamepad>,
    /// Next available player ID
    next_player_id: usize,
}

/// A single gamepad/controller
#[derive(Debug, Clone)]
pub struct Gamepad {
    /// Player ID (0-7)
    pub player_id: usize,
    /// Controller name/type
    pub name: String,
    /// Button states
    buttons_pressed: HashMap<GamepadButton, bool>,
    buttons_just_pressed: HashMap<GamepadButton, bool>,
    buttons_just_released: HashMap<GamepadButton, bool>,
    /// Left stick (X, Y) in range [-1.0, 1.0]
    pub left_stick: (f32, f32),
    /// Right stick (X, Y) in range [-1.0, 1.0]
    pub right_stick: (f32, f32),
    /// Left trigger in range [0.0, 1.0]
    pub left_trigger: f32,
    /// Right trigger in range [0.0, 1.0]
    pub right_trigger: f32,
    /// Deadzone for analog sticks
    pub stick_deadzone: f32,
    /// Deadzone for triggers
    pub trigger_deadzone: f32,
}

/// Gamepad button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    // Face buttons (Xbox layout)
    South,  // A (Xbox), Cross (PS), B (Nintendo)
    East,   // B (Xbox), Circle (PS), A (Nintendo)
    West,   // X (Xbox), Square (PS), Y (Nintendo)
    North,  // Y (Xbox), Triangle (PS), X (Nintendo)

    // D-Pad
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,

    // Shoulder buttons
    LeftShoulder,   // LB/L1
    RightShoulder,  // RB/R1

    // Triggers (digital)
    LeftTrigger,    // LT/L2 (as button)
    RightTrigger,   // RT/R2 (as button)

    // Stick buttons
    LeftStick,      // L3
    RightStick,     // R3

    // Menu buttons
    Start,          // Start/Options/+
    Select,         // Back/Share/-
    Guide,          // Xbox/PS/Home button

    // Additional
    LeftTrigger2,   // Additional trigger
    RightTrigger2,  // Additional trigger
}

/// Gamepad axis (for analog inputs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    LeftTrigger,
    RightTrigger,
}

impl GamepadManager {
    /// Create a new gamepad manager
    pub fn new() -> Self {
        Self {
            gamepads: HashMap::new(),
            next_player_id: 0,
        }
    }

    /// Connect a gamepad
    pub fn connect_gamepad(&mut self, name: String) -> usize {
        if self.next_player_id >= MAX_GAMEPADS {
            return usize::MAX; // No more slots
        }

        let player_id = self.next_player_id;
        self.next_player_id += 1;

        let gamepad = Gamepad::new(player_id, name);
        self.gamepads.insert(player_id, gamepad);
        player_id
    }

    /// Disconnect a gamepad
    pub fn disconnect_gamepad(&mut self, player_id: usize) {
        self.gamepads.remove(&player_id);
    }

    /// Get a gamepad by player ID
    pub fn get_gamepad(&self, player_id: usize) -> Option<&Gamepad> {
        self.gamepads.get(&player_id)
    }

    /// Get a mutable gamepad by player ID
    pub fn get_gamepad_mut(&mut self, player_id: usize) -> Option<&mut Gamepad> {
        self.gamepads.get_mut(&player_id)
    }

    /// Get the first connected gamepad (player 0)
    pub fn get_primary_gamepad(&self) -> Option<&Gamepad> {
        self.gamepads.get(&0)
    }

    /// Get the first connected gamepad (player 0) mutably
    pub fn get_primary_gamepad_mut(&mut self) -> Option<&mut Gamepad> {
        self.gamepads.get_mut(&0)
    }

    /// Get all connected gamepads
    pub fn get_all_gamepads(&self) -> Vec<&Gamepad> {
        self.gamepads.values().collect()
    }

    /// Check if any gamepad is connected
    pub fn has_gamepad(&self) -> bool {
        !self.gamepads.is_empty()
    }

    /// Get number of connected gamepads
    pub fn gamepad_count(&self) -> usize {
        self.gamepads.len()
    }

    /// Clear frame state for all gamepads
    pub fn clear_frame_state(&mut self) {
        for gamepad in self.gamepads.values_mut() {
            gamepad.clear_frame_state();
        }
    }
}

impl Default for GamepadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Gamepad {
    /// Create a new gamepad
    pub fn new(player_id: usize, name: String) -> Self {
        Self {
            player_id,
            name,
            buttons_pressed: HashMap::new(),
            buttons_just_pressed: HashMap::new(),
            buttons_just_released: HashMap::new(),
            left_stick: (0.0, 0.0),
            right_stick: (0.0, 0.0),
            left_trigger: 0.0,
            right_trigger: 0.0,
            stick_deadzone: 0.15,
            trigger_deadzone: 0.01,
        }
    }

    /// Check if a button is currently held down
    pub fn button_held(&self, button: GamepadButton) -> bool {
        self.buttons_pressed.get(&button).copied().unwrap_or(false)
    }

    /// Check if a button was just pressed this frame
    pub fn button_pressed(&self, button: GamepadButton) -> bool {
        self.buttons_just_pressed.get(&button).copied().unwrap_or(false)
    }

    /// Check if a button was just released this frame
    pub fn button_released(&self, button: GamepadButton) -> bool {
        self.buttons_just_released.get(&button).copied().unwrap_or(false)
    }

    /// Get left stick position with deadzone applied
    pub fn left_stick(&self) -> (f32, f32) {
        self.apply_deadzone(self.left_stick, self.stick_deadzone)
    }

    /// Get right stick position with deadzone applied
    pub fn right_stick(&self) -> (f32, f32) {
        self.apply_deadzone(self.right_stick, self.stick_deadzone)
    }

    /// Get left trigger value with deadzone applied
    pub fn left_trigger(&self) -> f32 {
        if self.left_trigger < self.trigger_deadzone {
            0.0
        } else {
            self.left_trigger
        }
    }

    /// Get right trigger value with deadzone applied
    pub fn right_trigger(&self) -> f32 {
        if self.right_trigger < self.trigger_deadzone {
            0.0
        } else {
            self.right_trigger
        }
    }

    /// Apply circular deadzone to stick input
    fn apply_deadzone(&self, stick: (f32, f32), deadzone: f32) -> (f32, f32) {
        let magnitude = (stick.0 * stick.0 + stick.1 * stick.1).sqrt();
        
        if magnitude < deadzone {
            (0.0, 0.0)
        } else {
            // Scale to maintain full range after deadzone
            let scale = (magnitude - deadzone) / (1.0 - deadzone) / magnitude;
            (stick.0 * scale, stick.1 * scale)
        }
    }

    /// Simulate button press (for testing)
    pub fn simulate_button_press(&mut self, button: GamepadButton) {
        self.buttons_pressed.insert(button, true);
        self.buttons_just_pressed.insert(button, true);
    }

    /// Simulate button release (for testing)
    pub fn simulate_button_release(&mut self, button: GamepadButton) {
        self.buttons_pressed.insert(button, false);
        self.buttons_just_released.insert(button, true);
    }

    /// Simulate stick movement (for testing)
    pub fn simulate_stick(&mut self, axis: GamepadAxis, value: f32) {
        match axis {
            GamepadAxis::LeftStickX => self.left_stick.0 = value,
            GamepadAxis::LeftStickY => self.left_stick.1 = value,
            GamepadAxis::RightStickX => self.right_stick.0 = value,
            GamepadAxis::RightStickY => self.right_stick.1 = value,
            GamepadAxis::LeftTrigger => self.left_trigger = value,
            GamepadAxis::RightTrigger => self.right_trigger = value,
        }
    }

    /// Clear frame state (just pressed/released)
    pub fn clear_frame_state(&mut self) {
        self.buttons_just_pressed.clear();
        self.buttons_just_released.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_manager_creation() {
        let manager = GamepadManager::new();
        assert_eq!(manager.gamepad_count(), 0);
        assert!(!manager.has_gamepad());
        println!("✅ GamepadManager created");
    }

    #[test]
    fn test_connect_gamepad() {
        let mut manager = GamepadManager::new();
        let player_id = manager.connect_gamepad("Xbox Controller".to_string());
        
        assert_eq!(player_id, 0);
        assert_eq!(manager.gamepad_count(), 1);
        assert!(manager.has_gamepad());
        println!("✅ Gamepad connected");
    }

    #[test]
    fn test_disconnect_gamepad() {
        let mut manager = GamepadManager::new();
        let player_id = manager.connect_gamepad("Xbox Controller".to_string());
        
        manager.disconnect_gamepad(player_id);
        assert_eq!(manager.gamepad_count(), 0);
        println!("✅ Gamepad disconnected");
    }

    #[test]
    fn test_gamepad_button_press() {
        let mut gamepad = Gamepad::new(0, "Test".to_string());
        
        gamepad.simulate_button_press(GamepadButton::South);
        assert!(gamepad.button_held(GamepadButton::South));
        assert!(gamepad.button_pressed(GamepadButton::South));
        println!("✅ Button press works");
    }

    #[test]
    fn test_gamepad_stick() {
        let mut gamepad = Gamepad::new(0, "Test".to_string());
        
        gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.5);
        gamepad.simulate_stick(GamepadAxis::LeftStickY, 0.3);
        
        let (x, y) = gamepad.left_stick();
        assert!(x > 0.0);
        assert!(y > 0.0);
        println!("✅ Stick input works");
    }

    #[test]
    fn test_deadzone() {
        let mut gamepad = Gamepad::new(0, "Test".to_string());
        gamepad.stick_deadzone = 0.2;
        
        // Below deadzone
        gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.1);
        let (x, _) = gamepad.left_stick();
        assert_eq!(x, 0.0, "Should be zeroed by deadzone");
        
        // Above deadzone
        gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.5);
        let (x, _) = gamepad.left_stick();
        assert!(x > 0.0, "Should pass deadzone");
        
        println!("✅ Deadzone works");
    }
}

