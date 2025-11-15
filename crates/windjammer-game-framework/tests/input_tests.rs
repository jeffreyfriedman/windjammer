//! Unit tests for Input System
//!
//! Tests keyboard and mouse input tracking, state management, and frame updates.

use windjammer_game_framework::input::{Input, Key, MouseButton};

// ============================================================================
// Input Creation Tests
// ============================================================================

#[test]
fn test_input_creation() {
    let input = Input::new();
    assert!(!input.held(Key::W));
    assert!(!input.pressed(Key::Space));
    println!("✅ Input created with empty state");
}

// ============================================================================
// Keyboard Tests - Basic State
// ============================================================================

#[test]
fn test_key_press() {
    let mut input = Input::new();
    
    // Simulate key press
    input.simulate_key_press(Key::W);
    
    assert!(input.held(Key::W), "Key should be held");
    assert!(input.pressed(Key::W), "Key should be just pressed");
    assert!(!input.released(Key::W), "Key should not be released");
    
    println!("✅ Key press detected");
}

#[test]
fn test_key_release() {
    let mut input = Input::new();
    
    // Press then release
    input.simulate_key_press(Key::W);
    input.clear_frame_state();
    input.simulate_key_release(Key::W);
    
    assert!(!input.held(Key::W), "Key should not be held");
    assert!(!input.pressed(Key::W), "Key should not be just pressed");
    assert!(input.released(Key::W), "Key should be just released");
    
    println!("✅ Key release detected");
}

#[test]
fn test_key_held_multiple_frames() {
    let mut input = Input::new();
    
    // Press key
    input.simulate_key_press(Key::W);
    assert!(input.held(Key::W));
    assert!(input.pressed(Key::W));
    
    // Update (next frame)
    input.clear_frame_state();
    assert!(input.held(Key::W), "Key should still be held");
    assert!(!input.pressed(Key::W), "Key should not be 'just pressed' anymore");
    
    // Another frame
    input.clear_frame_state();
    assert!(input.held(Key::W), "Key should still be held");
    assert!(!input.pressed(Key::W), "Key should not be 'just pressed'");
    
    println!("✅ Key held across multiple frames");
}

#[test]
fn test_multiple_keys() {
    let mut input = Input::new();
    
    input.simulate_key_press(Key::W);
    input.simulate_key_press(Key::A);
    input.simulate_key_press(Key::Space);
    
    assert!(input.held(Key::W));
    assert!(input.held(Key::A));
    assert!(input.held(Key::Space));
    assert!(!input.held(Key::S));
    
    println!("✅ Multiple keys tracked simultaneously");
}

// ============================================================================
// Keyboard Tests - Frame Updates
// ============================================================================

#[test]
fn test_just_pressed_clears_after_update() {
    let mut input = Input::new();
    
    input.simulate_key_press(Key::Space);
    assert!(input.pressed(Key::Space), "Should be just pressed");
    
    input.clear_frame_state();
    assert!(!input.pressed(Key::Space), "Should not be just pressed after update");
    assert!(input.held(Key::Space), "Should still be held");
    
    println!("✅ Just pressed clears after update");
}

#[test]
fn test_just_released_clears_after_update() {
    let mut input = Input::new();
    
    input.simulate_key_press(Key::Space);
    input.clear_frame_state();
    input.simulate_key_release(Key::Space);
    
    assert!(input.released(Key::Space), "Should be just released");
    
    input.clear_frame_state();
    assert!(!input.released(Key::Space), "Should not be just released after update");
    assert!(!input.held(Key::Space), "Should not be held");
    
    println!("✅ Just released clears after update");
}

#[test]
fn test_press_release_same_frame() {
    let mut input = Input::new();
    
    input.simulate_key_press(Key::W);
    input.simulate_key_release(Key::W);
    
    // In the same frame, both should be true
    assert!(!input.held(Key::W), "Key should not be held");
    assert!(input.released(Key::W), "Key should be released");
    
    println!("✅ Press and release in same frame handled correctly");
}

// ============================================================================
// Keyboard Tests - All Keys
// ============================================================================

#[test]
fn test_letter_keys() {
    let mut input = Input::new();
    
    let letters = vec![
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H,
        Key::I, Key::J, Key::K, Key::L, Key::M, Key::N, Key::O, Key::P,
        Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
        Key::Y, Key::Z,
    ];
    
    for key in letters {
        input.simulate_key_press(key);
        assert!(input.held(key), "Letter key {:?} should be held", key);
    }
    
    println!("✅ All letter keys work");
}

#[test]
fn test_number_keys() {
    let mut input = Input::new();
    
    let numbers = vec![
        Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4,
        Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9,
    ];
    
    for key in numbers {
        input.simulate_key_press(key);
        assert!(input.held(key), "Number key {:?} should be held", key);
    }
    
    println!("✅ All number keys work");
}

#[test]
fn test_arrow_keys() {
    let mut input = Input::new();
    
    input.simulate_key_press(Key::Up);
    input.simulate_key_press(Key::Down);
    input.simulate_key_press(Key::Left);
    input.simulate_key_press(Key::Right);
    
    assert!(input.held(Key::Up));
    assert!(input.held(Key::Down));
    assert!(input.held(Key::Left));
    assert!(input.held(Key::Right));
    
    println!("✅ Arrow keys work");
}

#[test]
fn test_special_keys() {
    let mut input = Input::new();
    
    let special = vec![
        Key::Space, Key::Enter, Key::Escape, Key::Tab,
        Key::Backspace, Key::Shift, Key::Control, Key::Alt,
    ];
    
    for key in special {
        input.simulate_key_press(key);
        assert!(input.held(key), "Special key {:?} should be held", key);
    }
    
    println!("✅ Special keys work");
}

#[test]
fn test_function_keys() {
    let mut input = Input::new();
    
    let function_keys = vec![
        Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6,
        Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12,
    ];
    
    for key in function_keys {
        input.simulate_key_press(key);
        assert!(input.held(key), "Function key {:?} should be held", key);
    }
    
    println!("✅ Function keys work");
}

// ============================================================================
// Mouse Tests - Buttons
// ============================================================================

#[test]
fn test_mouse_button_press() {
    let mut input = Input::new();
    
    input.simulate_mouse_press(MouseButton::Left);
    
    assert!(input.mouse_held(MouseButton::Left));
    assert!(input.mouse_pressed(MouseButton::Left));
    assert!(!input.mouse_released(MouseButton::Left));
    
    println!("✅ Mouse button press detected");
}

#[test]
fn test_mouse_button_release() {
    let mut input = Input::new();
    
    input.simulate_mouse_press(MouseButton::Left);
    input.clear_frame_state();
    input.simulate_mouse_release(MouseButton::Left);
    
    assert!(!input.mouse_held(MouseButton::Left));
    assert!(!input.mouse_pressed(MouseButton::Left));
    assert!(input.mouse_released(MouseButton::Left));
    
    println!("✅ Mouse button release detected");
}

#[test]
fn test_all_mouse_buttons() {
    let mut input = Input::new();
    
    input.simulate_mouse_press(MouseButton::Left);
    input.simulate_mouse_press(MouseButton::Right);
    input.simulate_mouse_press(MouseButton::Middle);
    
    assert!(input.mouse_held(MouseButton::Left));
    assert!(input.mouse_held(MouseButton::Right));
    assert!(input.mouse_held(MouseButton::Middle));
    
    println!("✅ All mouse buttons work");
}

// ============================================================================
// Mouse Tests - Position & Delta
// ============================================================================

#[test]
fn test_mouse_position() {
    let mut input = Input::new();
    
    input.simulate_mouse_move(100.0, 200.0);
    
    let (x, y) = input.mouse_position();
    assert_eq!(x, 100.0);
    assert_eq!(y, 200.0);
    
    println!("✅ Mouse position tracked: ({}, {})", x, y);
}

#[test]
fn test_mouse_delta() {
    let mut input = Input::new();
    
    // Initial position
    input.simulate_mouse_move(100.0, 100.0);
    input.clear_frame_state();
    
    // Move mouse
    input.simulate_mouse_move(150.0, 120.0);
    
    let (dx, dy) = input.mouse_delta();
    assert_eq!(dx, 50.0, "Delta X should be 50");
    assert_eq!(dy, 20.0, "Delta Y should be 20");
    
    println!("✅ Mouse delta tracked: ({}, {})", dx, dy);
}

#[test]
fn test_mouse_delta_resets_after_update() {
    let mut input = Input::new();
    
    input.simulate_mouse_move(100.0, 100.0);
    input.clear_frame_state();
    input.simulate_mouse_move(150.0, 120.0);
    
    let (dx1, dy1) = input.mouse_delta();
    assert_eq!(dx1, 50.0);
    assert_eq!(dy1, 20.0);
    
    // After update, delta should be based on new position
    input.clear_frame_state();
    input.simulate_mouse_move(150.0, 120.0); // Same position
    
    let (dx2, dy2) = input.mouse_delta();
    assert_eq!(dx2, 0.0, "Delta should be 0 when mouse doesn't move");
    assert_eq!(dy2, 0.0, "Delta should be 0 when mouse doesn't move");
    
    println!("✅ Mouse delta resets correctly");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_game_loop_simulation() {
    let mut input = Input::new();
    
    // Frame 1: Press W
    input.simulate_key_press(Key::W);
    assert!(input.held(Key::W));
    assert!(input.pressed(Key::W));
    
    // Frame 2: W still held
    input.clear_frame_state();
    assert!(input.held(Key::W));
    assert!(!input.pressed(Key::W));
    
    // Frame 3: Press Space while holding W
    input.clear_frame_state();
    input.simulate_key_press(Key::Space);
    assert!(input.held(Key::W));
    assert!(input.held(Key::Space));
    assert!(!input.pressed(Key::W));
    assert!(input.pressed(Key::Space));
    
    // Frame 4: Release W
    input.clear_frame_state();
    input.simulate_key_release(Key::W);
    assert!(!input.held(Key::W));
    assert!(input.released(Key::W));
    assert!(input.held(Key::Space));
    
    // Frame 5: All clear
    input.clear_frame_state();
    assert!(!input.held(Key::W));
    assert!(!input.released(Key::W));
    assert!(input.held(Key::Space));
    
    println!("✅ Game loop simulation works correctly");
}

#[test]
fn test_wasd_movement() {
    let mut input = Input::new();
    
    // Simulate WASD movement
    input.simulate_key_press(Key::W); // Forward
    assert!(input.held(Key::W));
    
    input.clear_frame_state();
    input.simulate_key_press(Key::A); // Forward + Left
    assert!(input.held(Key::W));
    assert!(input.held(Key::A));
    
    input.clear_frame_state();
    input.simulate_key_release(Key::W); // Just Left
    assert!(!input.held(Key::W));
    assert!(input.held(Key::A));
    
    println!("✅ WASD movement tracking works");
}

#[test]
fn test_fps_controls() {
    let mut input = Input::new();
    
    // WASD movement
    input.simulate_key_press(Key::W);
    input.simulate_key_press(Key::A);
    
    // Mouse look
    input.simulate_mouse_move(100.0, 100.0);
    input.clear_frame_state();
    input.simulate_mouse_move(150.0, 110.0);
    
    // Jump
    input.simulate_key_press(Key::Space);
    
    // Shoot
    input.simulate_mouse_press(MouseButton::Left);
    
    assert!(input.held(Key::W));
    assert!(input.held(Key::A));
    assert!(input.pressed(Key::Space));
    assert!(input.mouse_pressed(MouseButton::Left));
    
    let (dx, dy) = input.mouse_delta();
    assert!(dx > 0.0);
    assert!(dy > 0.0);
    
    println!("✅ FPS controls work");
}

#[test]
fn test_frame_state_clear() {
    let mut input = Input::new();
    
    // Press keys
    input.simulate_key_press(Key::W);
    input.simulate_mouse_press(MouseButton::Left);
    
    assert!(input.pressed(Key::W));
    assert!(input.mouse_pressed(MouseButton::Left));
    
    // Clear frame state
    input.clear_frame_state();
    
    // "Just pressed" should be cleared, but "held" should remain
    assert!(!input.pressed(Key::W), "Just pressed should be cleared");
    assert!(input.held(Key::W), "Held should remain");
    assert!(!input.mouse_pressed(MouseButton::Left), "Just pressed should be cleared");
    assert!(input.mouse_held(MouseButton::Left), "Held should remain");
    
    println!("✅ Frame state clear works");
}

