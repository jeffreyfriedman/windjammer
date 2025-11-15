//! Unit tests for Gamepad System
//!
//! Tests gamepad connection, buttons, analog inputs, and deadzone handling.

use windjammer_game_framework::gamepad::*;

// ============================================================================
// GamepadManager Tests
// ============================================================================

#[test]
fn test_gamepad_manager_creation() {
    let manager = GamepadManager::new();
    assert_eq!(manager.gamepad_count(), 0);
    assert!(!manager.has_gamepad());
    println!("✅ GamepadManager created");
}

#[test]
fn test_gamepad_manager_default() {
    let manager = GamepadManager::default();
    assert_eq!(manager.gamepad_count(), 0);
    println!("✅ GamepadManager default");
}

#[test]
fn test_connect_single_gamepad() {
    let mut manager = GamepadManager::new();
    let player_id = manager.connect_gamepad("Xbox Controller".to_string());
    
    assert_eq!(player_id, 0, "First gamepad should be player 0");
    assert_eq!(manager.gamepad_count(), 1);
    assert!(manager.has_gamepad());
    println!("✅ Single gamepad connected");
}

#[test]
fn test_connect_multiple_gamepads() {
    let mut manager = GamepadManager::new();
    
    let id1 = manager.connect_gamepad("Xbox Controller 1".to_string());
    let id2 = manager.connect_gamepad("Xbox Controller 2".to_string());
    let id3 = manager.connect_gamepad("PS5 Controller".to_string());
    
    assert_eq!(id1, 0);
    assert_eq!(id2, 1);
    assert_eq!(id3, 2);
    assert_eq!(manager.gamepad_count(), 3);
    println!("✅ Multiple gamepads connected");
}

#[test]
fn test_max_gamepads() {
    let mut manager = GamepadManager::new();
    
    // Connect MAX_GAMEPADS controllers
    for i in 0..MAX_GAMEPADS {
        let id = manager.connect_gamepad(format!("Controller {}", i));
        assert_eq!(id, i);
    }
    
    // Try to connect one more (should fail)
    let overflow_id = manager.connect_gamepad("Overflow Controller".to_string());
    assert_eq!(overflow_id, usize::MAX, "Should reject connection beyond max");
    assert_eq!(manager.gamepad_count(), MAX_GAMEPADS);
    
    println!("✅ Max gamepads limit enforced: {}", MAX_GAMEPADS);
}

#[test]
fn test_disconnect_gamepad() {
    let mut manager = GamepadManager::new();
    let player_id = manager.connect_gamepad("Xbox Controller".to_string());
    
    assert_eq!(manager.gamepad_count(), 1);
    
    manager.disconnect_gamepad(player_id);
    assert_eq!(manager.gamepad_count(), 0);
    assert!(!manager.has_gamepad());
    println!("✅ Gamepad disconnected");
}

#[test]
fn test_get_gamepad() {
    let mut manager = GamepadManager::new();
    let player_id = manager.connect_gamepad("Xbox Controller".to_string());
    
    let gamepad = manager.get_gamepad(player_id);
    assert!(gamepad.is_some());
    assert_eq!(gamepad.unwrap().player_id, player_id);
    println!("✅ Get gamepad by ID works");
}

#[test]
fn test_get_primary_gamepad() {
    let mut manager = GamepadManager::new();
    manager.connect_gamepad("Primary Controller".to_string());
    
    let primary = manager.get_primary_gamepad();
    assert!(primary.is_some());
    assert_eq!(primary.unwrap().player_id, 0);
    println!("✅ Get primary gamepad works");
}

#[test]
fn test_get_all_gamepads() {
    let mut manager = GamepadManager::new();
    manager.connect_gamepad("Controller 1".to_string());
    manager.connect_gamepad("Controller 2".to_string());
    manager.connect_gamepad("Controller 3".to_string());
    
    let all = manager.get_all_gamepads();
    assert_eq!(all.len(), 3);
    println!("✅ Get all gamepads works");
}

// ============================================================================
// Gamepad Tests
// ============================================================================

#[test]
fn test_gamepad_creation() {
    let gamepad = Gamepad::new(0, "Xbox Controller".to_string());
    assert_eq!(gamepad.player_id, 0);
    assert_eq!(gamepad.name, "Xbox Controller");
    assert_eq!(gamepad.left_stick, (0.0, 0.0));
    assert_eq!(gamepad.right_stick, (0.0, 0.0));
    assert_eq!(gamepad.left_trigger, 0.0);
    assert_eq!(gamepad.right_trigger, 0.0);
    println!("✅ Gamepad created");
}

// ============================================================================
// Button Tests
// ============================================================================

#[test]
fn test_button_press() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    gamepad.simulate_button_press(GamepadButton::South);
    
    assert!(gamepad.button_held(GamepadButton::South), "Button should be held");
    assert!(gamepad.button_pressed(GamepadButton::South), "Button should be just pressed");
    assert!(!gamepad.button_released(GamepadButton::South), "Button should not be released");
    
    println!("✅ Button press works");
}

#[test]
fn test_button_release() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    // Press then release
    gamepad.simulate_button_press(GamepadButton::South);
    gamepad.simulate_button_release(GamepadButton::South);
    
    assert!(!gamepad.button_held(GamepadButton::South), "Button should not be held");
    assert!(gamepad.button_released(GamepadButton::South), "Button should be just released");
    
    println!("✅ Button release works");
}

#[test]
fn test_multiple_buttons() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    gamepad.simulate_button_press(GamepadButton::South);
    gamepad.simulate_button_press(GamepadButton::East);
    gamepad.simulate_button_press(GamepadButton::West);
    
    assert!(gamepad.button_held(GamepadButton::South));
    assert!(gamepad.button_held(GamepadButton::East));
    assert!(gamepad.button_held(GamepadButton::West));
    assert!(!gamepad.button_held(GamepadButton::North));
    
    println!("✅ Multiple buttons work");
}

#[test]
fn test_all_button_types() {
    let buttons = vec![
        GamepadButton::South,
        GamepadButton::East,
        GamepadButton::West,
        GamepadButton::North,
        GamepadButton::DPadUp,
        GamepadButton::DPadDown,
        GamepadButton::DPadLeft,
        GamepadButton::DPadRight,
        GamepadButton::LeftShoulder,
        GamepadButton::RightShoulder,
        GamepadButton::LeftTrigger,
        GamepadButton::RightTrigger,
        GamepadButton::LeftStick,
        GamepadButton::RightStick,
        GamepadButton::Start,
        GamepadButton::Select,
        GamepadButton::Guide,
    ];
    
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    for button in &buttons {
        gamepad.simulate_button_press(*button);
        assert!(gamepad.button_held(*button), "Button {:?} should work", button);
    }
    
    println!("✅ All button types work: {} buttons", buttons.len());
}

// ============================================================================
// Analog Stick Tests
// ============================================================================

#[test]
fn test_left_stick() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.5);
    gamepad.simulate_stick(GamepadAxis::LeftStickY, 0.3);
    
    let (x, y) = gamepad.left_stick();
    assert!(x > 0.0, "Left stick X should be positive");
    assert!(y > 0.0, "Left stick Y should be positive");
    
    println!("✅ Left stick works: ({}, {})", x, y);
}

#[test]
fn test_right_stick() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    gamepad.simulate_stick(GamepadAxis::RightStickX, -0.7);
    gamepad.simulate_stick(GamepadAxis::RightStickY, 0.9);
    
    let (x, y) = gamepad.right_stick();
    assert!(x < 0.0, "Right stick X should be negative");
    assert!(y > 0.0, "Right stick Y should be positive");
    
    println!("✅ Right stick works: ({}, {})", x, y);
}

#[test]
fn test_stick_range() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    gamepad.stick_deadzone = 0.0; // Disable deadzone for this test
    
    // Test full range
    gamepad.simulate_stick(GamepadAxis::LeftStickX, 1.0);
    let (x, _) = gamepad.left_stick();
    assert!((x - 1.0).abs() < 0.01, "Stick should reach maximum");
    
    gamepad.simulate_stick(GamepadAxis::LeftStickX, -1.0);
    let (x, _) = gamepad.left_stick();
    assert!((x - -1.0).abs() < 0.01, "Stick should reach minimum");
    
    println!("✅ Stick range works: -1.0 to 1.0");
}

// ============================================================================
// Trigger Tests
// ============================================================================

#[test]
fn test_left_trigger() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    gamepad.simulate_stick(GamepadAxis::LeftTrigger, 0.75);
    
    let value = gamepad.left_trigger();
    assert!(value > 0.0, "Left trigger should be positive");
    assert!(value <= 1.0, "Left trigger should be <= 1.0");
    
    println!("✅ Left trigger works: {}", value);
}

#[test]
fn test_right_trigger() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    gamepad.simulate_stick(GamepadAxis::RightTrigger, 0.5);
    
    let value = gamepad.right_trigger();
    assert!(value > 0.0, "Right trigger should be positive");
    
    println!("✅ Right trigger works: {}", value);
}

#[test]
fn test_trigger_range() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    gamepad.trigger_deadzone = 0.0; // Disable deadzone
    
    // Test full range
    gamepad.simulate_stick(GamepadAxis::LeftTrigger, 0.0);
    assert_eq!(gamepad.left_trigger(), 0.0, "Trigger should be at minimum");
    
    gamepad.simulate_stick(GamepadAxis::LeftTrigger, 1.0);
    assert_eq!(gamepad.left_trigger(), 1.0, "Trigger should be at maximum");
    
    println!("✅ Trigger range works: 0.0 to 1.0");
}

// ============================================================================
// Deadzone Tests
// ============================================================================

#[test]
fn test_stick_deadzone() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    gamepad.stick_deadzone = 0.2;
    
    // Below deadzone
    gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.1);
    gamepad.simulate_stick(GamepadAxis::LeftStickY, 0.1);
    let (x, y) = gamepad.left_stick();
    assert_eq!(x, 0.0, "Should be zeroed by deadzone");
    assert_eq!(y, 0.0, "Should be zeroed by deadzone");
    
    // Above deadzone
    gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.5);
    gamepad.simulate_stick(GamepadAxis::LeftStickY, 0.5);
    let (x, y) = gamepad.left_stick();
    assert!(x > 0.0, "Should pass deadzone");
    assert!(y > 0.0, "Should pass deadzone");
    
    println!("✅ Stick deadzone works: threshold={}", gamepad.stick_deadzone);
}

#[test]
fn test_trigger_deadzone() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    gamepad.trigger_deadzone = 0.05;
    
    // Below deadzone
    gamepad.simulate_stick(GamepadAxis::LeftTrigger, 0.03);
    assert_eq!(gamepad.left_trigger(), 0.0, "Should be zeroed by deadzone");
    
    // Above deadzone
    gamepad.simulate_stick(GamepadAxis::LeftTrigger, 0.5);
    assert!(gamepad.left_trigger() > 0.0, "Should pass deadzone");
    
    println!("✅ Trigger deadzone works: threshold={}", gamepad.trigger_deadzone);
}

#[test]
fn test_circular_deadzone() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    gamepad.stick_deadzone = 0.2;
    
    // Diagonal input within deadzone circle
    gamepad.simulate_stick(GamepadAxis::LeftStickX, 0.1);
    gamepad.simulate_stick(GamepadAxis::LeftStickY, 0.1);
    let (x, y) = gamepad.left_stick();
    
    // Magnitude is sqrt(0.1^2 + 0.1^2) = 0.141, which is < 0.2
    assert_eq!(x, 0.0, "Diagonal should be zeroed by circular deadzone");
    assert_eq!(y, 0.0, "Diagonal should be zeroed by circular deadzone");
    
    println!("✅ Circular deadzone works");
}

// ============================================================================
// Frame State Tests
// ============================================================================

#[test]
fn test_clear_frame_state() {
    let mut gamepad = Gamepad::new(0, "Test".to_string());
    
    // Press button
    gamepad.simulate_button_press(GamepadButton::South);
    assert!(gamepad.button_pressed(GamepadButton::South), "Should be just pressed");
    
    // Clear frame state
    gamepad.clear_frame_state();
    
    // "Just pressed" should be cleared, but "held" should remain
    assert!(!gamepad.button_pressed(GamepadButton::South), "Just pressed should be cleared");
    assert!(gamepad.button_held(GamepadButton::South), "Held should remain");
    
    println!("✅ Frame state clear works");
}

#[test]
fn test_manager_clear_frame_state() {
    let mut manager = GamepadManager::new();
    let id1 = manager.connect_gamepad("Controller 1".to_string());
    let id2 = manager.connect_gamepad("Controller 2".to_string());
    
    // Press buttons on both gamepads
    manager.get_gamepad_mut(id1).unwrap().simulate_button_press(GamepadButton::South);
    manager.get_gamepad_mut(id2).unwrap().simulate_button_press(GamepadButton::East);
    
    // Clear all
    manager.clear_frame_state();
    
    // Check that both were cleared
    assert!(!manager.get_gamepad(id1).unwrap().button_pressed(GamepadButton::South));
    assert!(!manager.get_gamepad(id2).unwrap().button_pressed(GamepadButton::East));
    
    println!("✅ Manager frame state clear works");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_multiplayer_scenario() {
    let mut manager = GamepadManager::new();
    
    // Connect 4 players
    let p1 = manager.connect_gamepad("P1 Controller".to_string());
    let p2 = manager.connect_gamepad("P2 Controller".to_string());
    let p3 = manager.connect_gamepad("P3 Controller".to_string());
    let p4 = manager.connect_gamepad("P4 Controller".to_string());
    
    // Each player presses a different button
    manager.get_gamepad_mut(p1).unwrap().simulate_button_press(GamepadButton::South);
    manager.get_gamepad_mut(p2).unwrap().simulate_button_press(GamepadButton::East);
    manager.get_gamepad_mut(p3).unwrap().simulate_button_press(GamepadButton::West);
    manager.get_gamepad_mut(p4).unwrap().simulate_button_press(GamepadButton::North);
    
    // Verify each player's input
    assert!(manager.get_gamepad(p1).unwrap().button_held(GamepadButton::South));
    assert!(manager.get_gamepad(p2).unwrap().button_held(GamepadButton::East));
    assert!(manager.get_gamepad(p3).unwrap().button_held(GamepadButton::West));
    assert!(manager.get_gamepad(p4).unwrap().button_held(GamepadButton::North));
    
    println!("✅ Multiplayer scenario works: 4 players");
}

#[test]
fn test_hot_plug() {
    let mut manager = GamepadManager::new();
    
    // Start with no gamepads
    assert_eq!(manager.gamepad_count(), 0);
    
    // Player 1 connects
    let p1 = manager.connect_gamepad("P1".to_string());
    assert_eq!(manager.gamepad_count(), 1);
    
    // Player 2 connects
    let p2 = manager.connect_gamepad("P2".to_string());
    assert_eq!(manager.gamepad_count(), 2);
    
    // Player 1 disconnects
    manager.disconnect_gamepad(p1);
    assert_eq!(manager.gamepad_count(), 1);
    
    // Player 3 connects
    let _p3 = manager.connect_gamepad("P3".to_string());
    assert_eq!(manager.gamepad_count(), 2);
    
    // Player 2 still works
    assert!(manager.get_gamepad(p2).is_some());
    
    println!("✅ Hot-plug scenario works");
}

