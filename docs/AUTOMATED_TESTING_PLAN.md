# 🧪 Automated Testing System for Windjammer Game Framework

**Date:** November 9, 2025  
**Status:** 📋 Planning Phase  
**Priority:** 🔴 **CRITICAL** - Prevents bugs from reaching users

**IMPORTANT:** Windjammer ALREADY HAS a complete test framework (`wj test`)!  
We just need to add game-specific testing utilities.

---

## 🎯 Motivation

**Current Problem:**
- User reported 3 critical bugs in the shooter game:
  1. A and D controls are backwards
  2. Mouse look doesn't work
  3. Shooting doesn't work

**Root Cause:**
- No automated testing or playtesting before user testing
- Manual testing is time-consuming and error-prone
- No way to catch regressions

**Goal:**
Create a comprehensive automated testing system that can:
1. **Compile-test** all examples automatically
2. **Runtime-test** game logic without manual interaction
3. **Integration-test** the full game framework
4. **Regression-test** to prevent breaking existing functionality

---

## ✅ What Windjammer ALREADY HAS

Windjammer has a **complete test framework**:
- ✅ `wj test` command
- ✅ Test discovery (`*_test.wj` files)
- ✅ Test functions (`test_*` prefix)
- ✅ `@test` decorator
- ✅ Assertions (`assert()`, `assert_eq()`)
- ✅ Parallel execution
- ✅ Test filtering
- ✅ JSON output for CI/CD

**Example:**
```windjammer
// tests/math_test.wj

fn test_addition() {
    let result = 2 + 2
    assert(result == 4, "2 + 2 should equal 4")
}

fn test_multiplication() {
    let result = 3 * 4
    assert(result == 12, "3 * 4 should equal 12")
}
```

Run with: `wj test`

---

## ❌ What We're MISSING (Game-Specific)

To test games in pure Windjammer, we need:

### 1. **Headless Mode**
Games currently require a window and GPU. We need a headless mode that:
- Skips window creation
- Skips rendering
- Still runs game logic
- Allows testing without a display

### 2. **Input Simulation API**
The `Input` struct needs test-friendly methods:
```windjammer
input.simulate_key_press(Key::W)
input.simulate_mouse_move(100.0, 0.0)
input.simulate_mouse_click(MouseButton::Left)
```

### 3. **Test-Friendly Initialization**
Games need a way to initialize without rendering:
```windjammer
let game = ShooterGame::new_headless()
// or
let game = ShooterGame::default()
game.set_headless(true)
```

### 4. **Deterministic Time**
Tests need predictable delta times:
```windjammer
game.update(0.016, input)  // Always 16ms
```

---

## 🔬 Research: Game Testing Approaches

### 1. **Headless Testing** (No GPU/Window)
**Pros:**
- Fast, can run in CI/CD
- No manual interaction needed
- Deterministic results

**Cons:**
- Can't test rendering
- Can't test input handling fully

**Use Cases:**
- Game logic (physics, collision, AI)
- State management
- Math utilities

### 2. **Automated Input Simulation**
**Pros:**
- Tests full input pipeline
- Can catch input bugs (like A/D backwards)
- Can test game flow

**Cons:**
- Requires window/event loop
- Slower than headless
- May need virtual display in CI

**Use Cases:**
- Input handling
- Player movement
- Weapon switching

### 3. **Visual Regression Testing**
**Pros:**
- Catches rendering bugs
- Ensures visual consistency
- Can detect missing elements

**Cons:**
- Requires GPU
- Slow
- Brittle (sensitive to small changes)

**Use Cases:**
- Rendering correctness
- UI layout
- Visual effects

### 4. **Property-Based Testing**
**Pros:**
- Finds edge cases automatically
- Tests invariants
- Comprehensive coverage

**Cons:**
- Requires careful property design
- Can be slow
- May find irrelevant bugs

**Use Cases:**
- Physics invariants (e.g., energy conservation)
- Collision detection correctness
- Math utilities

---

## 🏗️ Proposed Architecture

### Layer 1: Unit Tests (Rust)
**Location:** `crates/windjammer-game-framework/tests/`

**What to Test:**
- Math utilities (Vec3, Mat4, quaternions)
- Input state management
- Renderer initialization
- Asset loading
- ECS operations

**Tools:**
- Standard Rust `#[test]`
- `cargo test`

**Example:**
```rust
#[test]
fn test_vec3_normalize() {
    let v = Vec3::new(3.0, 4.0, 0.0);
    let normalized = v.normalize();
    assert!((normalized.length() - 1.0).abs() < 0.001);
}

#[test]
fn test_input_key_pressed() {
    let mut input = Input::new();
    input.update_from_winit(&KeyEvent { /* ... */ });
    assert!(input.pressed(Key::W));
}
```

---

### Layer 2: Integration Tests (Windjammer)
**Location:** `tests/game_framework/`

**What to Test:**
- Full game loop (without rendering)
- Player movement logic
- Enemy AI
- Collision detection
- Weapon firing

**Tools:**
- Windjammer test framework (to be built)
- Headless mode for game framework

**Example:**
```windjammer
@test
fn test_player_movement() {
    let game = ShooterGame::default()
    let input = Input::new()
    
    // Simulate W key press
    input.press(Key::W)
    
    // Update game
    game.update(0.016, input)
    
    // Check player moved forward
    assert(game.player_pos.z > 0.0, "Player should move forward when W is pressed")
}

@test
fn test_shooting() {
    let game = ShooterGame::default()
    let input = Input::new()
    
    // Simulate mouse click
    input.press_mouse(MouseButton::Left)
    
    // Update game
    game.update(0.016, input)
    
    // Check bullet was spawned
    assert(game.bullets.len() == 1, "Bullet should be spawned on mouse click")
}
```

---

### Layer 3: Automated Playtest (Simulated)
**Location:** `tests/playtest/`

**What to Test:**
- Full game scenarios
- Win/lose conditions
- Edge cases (e.g., running into walls)

**Tools:**
- Scripted input sequences
- Headless rendering (optional)
- Deterministic random seed

**Example:**
```windjammer
@playtest
fn test_kill_all_enemies() {
    let game = ShooterGame::default()
    let script = PlaytestScript::new()
    
    // Script: Move to enemy, shoot, repeat
    script.move_to(Vec3::new(10.0, 0.0, 10.0))
    script.aim_at_enemy(0)
    script.shoot()
    script.wait(1.0)
    
    // Run script
    let result = game.run_script(script)
    
    // Check win condition
    assert(result.enemies_killed == 5, "Should kill all 5 enemies")
    assert(result.won == true, "Should win the game")
}
```

---

### Layer 4: Visual Regression Tests
**Location:** `tests/visual/`

**What to Test:**
- Rendering output
- UI elements
- Visual effects

**Tools:**
- Screenshot comparison
- Pixel-perfect diff
- Perceptual diff (for anti-aliasing)

**Example:**
```rust
#[test]
fn test_pong_rendering() {
    let game = PongGame::default();
    let screenshot = render_to_image(&game);
    
    // Compare with golden image
    let golden = load_image("tests/visual/pong_initial.png");
    let diff = compare_images(&screenshot, &golden);
    
    assert!(diff < 0.01, "Rendering should match golden image");
}
```

---

## 🛠️ Implementation Plan

### Phase 1: Foundation (Week 1)
**Goal:** Basic test infrastructure

**Tasks:**
1. Add `#[test]` support to Windjammer parser
2. Create `wj test` command
3. Implement headless mode for game framework
4. Write 10 unit tests for math utilities

**Deliverables:**
- `wj test` command works
- Math utilities are fully tested
- CI runs tests automatically

---

### Phase 2: Input Testing (Week 2)
**Goal:** Test input handling

**Tasks:**
1. Add mouse support to Input system
2. Create input simulation API
3. Write tests for keyboard input
4. Write tests for mouse input
5. Test PONG game controls

**Deliverables:**
- Input system is fully tested
- PONG game has automated tests
- Catches input bugs automatically

---

### Phase 3: Game Logic Testing (Week 3)
**Goal:** Test game logic without rendering

**Tasks:**
1. Create headless game runner
2. Write tests for shooter game logic
3. Test player movement
4. Test enemy AI
5. Test collision detection
6. Test shooting mechanics

**Deliverables:**
- Shooter game logic is fully tested
- Catches logic bugs before user testing
- Can run tests in CI

---

### Phase 4: Integration Testing (Week 4)
**Goal:** Test full game scenarios

**Tasks:**
1. Create playtest scripting API
2. Write end-to-end tests
3. Test win/lose conditions
4. Test edge cases
5. Add property-based tests

**Deliverables:**
- Full game scenarios are tested
- Edge cases are covered
- Property-based tests find bugs

---

### Phase 5: Visual Testing (Week 5)
**Goal:** Test rendering output

**Tasks:**
1. Implement screenshot capture
2. Create golden image library
3. Write visual regression tests
4. Add perceptual diff
5. Test all examples

**Deliverables:**
- Rendering is tested automatically
- Visual regressions are caught
- All examples have visual tests

---

## 🎮 Specific Tests for Shooter Game

### Input Tests
```windjammer
@test
fn test_wasd_movement() {
    let game = ShooterGame::default()
    let input = Input::new()
    
    // Test W (forward)
    input.press(Key::W)
    game.update(0.016, input)
    let forward_z = game.player_pos.z
    assert(forward_z > 0.0, "W should move forward (+Z)")
    
    // Test S (backward)
    game.reset()
    input.press(Key::S)
    game.update(0.016, input)
    let backward_z = game.player_pos.z
    assert(backward_z < 0.0, "S should move backward (-Z)")
    
    // Test A (left)
    game.reset()
    input.press(Key::A)
    game.update(0.016, input)
    let left_x = game.player_pos.x
    assert(left_x < 0.0, "A should move left (-X)")
    
    // Test D (right)
    game.reset()
    input.press(Key::D)
    game.update(0.016, input)
    let right_x = game.player_pos.x
    assert(right_x > 0.0, "D should move right (+X)")
}

@test
fn test_mouse_look() {
    let game = ShooterGame::default()
    let input = Input::new()
    
    // Simulate mouse movement right
    input.move_mouse_delta(100.0, 0.0)
    game.update(0.016, input)
    
    // Check yaw increased
    assert(game.player_yaw > 0.0, "Mouse right should increase yaw")
    
    // Simulate mouse movement up
    game.reset()
    input.move_mouse_delta(0.0, -100.0)
    game.update(0.016, input)
    
    // Check pitch increased
    assert(game.player_pitch > 0.0, "Mouse up should increase pitch")
}

@test
fn test_shooting() {
    let game = ShooterGame::default()
    let input = Input::new()
    
    // Simulate left click
    input.press_mouse(MouseButton::Left)
    game.update(0.016, input)
    
    // Check bullet spawned
    assert(game.bullets.len() == 1, "Left click should spawn bullet")
    
    // Check bullet direction
    let bullet = game.bullets[0]
    assert(bullet.velocity.z > 0.0, "Bullet should move forward")
}
```

### Physics Tests
```windjammer
@test
fn test_gravity() {
    let game = ShooterGame::default()
    game.player_pos.y = 10.0
    game.player_on_ground = false
    
    // Update for 1 second
    for i in 0..60 {
        game.update(0.016, Input::new())
    }
    
    // Check player fell to ground
    assert(game.player_pos.y == 2.0, "Player should fall to ground")
    assert(game.player_on_ground == true, "Player should be on ground")
}

@test
fn test_collision() {
    let game = ShooterGame::default()
    game.player_pos = Vec3::new(0.0, 2.0, 0.0)
    
    // Try to move into wall at x=20
    let input = Input::new()
    input.press(Key::D)
    
    // Update for 10 seconds (should hit wall)
    for i in 0..600 {
        game.update(0.016, input)
    }
    
    // Check player stopped at wall
    assert(game.player_pos.x < 20.0, "Player should not go through wall")
}
```

### AI Tests
```windjammer
@test
fn test_enemy_chase() {
    let game = ShooterGame::default()
    game.player_pos = Vec3::new(10.0, 2.0, 10.0)
    
    let enemy = game.enemies[0]
    let initial_dist = distance(enemy.pos, game.player_pos)
    
    // Update for 5 seconds
    for i in 0..300 {
        game.update(0.016, Input::new())
    }
    
    // Check enemy moved closer
    let final_dist = distance(game.enemies[0].pos, game.player_pos)
    assert(final_dist < initial_dist, "Enemy should chase player")
}
```

---

## 🚀 Quick Wins (Immediate)

### 1. Add Compile Tests
**File:** `tests/compile_all_examples.sh`
```bash
#!/bin/bash
set -e

echo "Testing all examples compile..."

for example in examples/**/*.wj; do
    echo "Compiling $example..."
    ./target/release/wj build "$example"
    cd build
    cargo check
    cd ..
done

echo "✅ All examples compile!"
```

### 2. Add Manual Test Checklist
**File:** `docs/MANUAL_TESTING_CHECKLIST.md`
```markdown
# Manual Testing Checklist

## PONG Game
- [ ] Left paddle moves up with W
- [ ] Left paddle moves down with S
- [ ] Right paddle moves up with Up
- [ ] Right paddle moves down with Down
- [ ] Ball bounces off paddles
- [ ] Ball bounces off top/bottom
- [ ] Score increases when ball goes off screen
- [ ] Center line is visible

## Shooter Game
- [ ] W moves forward (+Z)
- [ ] S moves backward (-Z)
- [ ] A moves left (-X)
- [ ] D moves right (+X)
- [ ] Mouse left/right rotates camera (yaw)
- [ ] Mouse up/down tilts camera (pitch)
- [ ] Left click shoots
- [ ] Bullets spawn in front of player
- [ ] Bullets hit enemies
- [ ] Enemies chase player
- [ ] Collision with walls works
- [ ] Gravity works
- [ ] Jump works
```

### 3. Add Debug Logging
**File:** `crates/windjammer-game-framework/src/debug.rs`
```rust
pub fn log_input(input: &Input) {
    println!("Keys pressed: {:?}", input.keys_pressed);
    println!("Mouse buttons: {:?}", input.mouse_buttons_pressed);
    println!("Mouse delta: {:?}", input.mouse_delta);
}

pub fn log_player_state(game: &ShooterGame) {
    println!("Player pos: {:?}", game.player_pos);
    println!("Player yaw: {}", game.player_yaw);
    println!("Player pitch: {}", game.player_pitch);
    println!("Bullets: {}", game.bullets.len());
}
```

---

## 📊 Success Metrics

### Coverage Goals
- **Unit Tests:** 80% code coverage
- **Integration Tests:** All major features
- **Visual Tests:** All examples

### Performance Goals
- **Unit Tests:** < 1 second total
- **Integration Tests:** < 10 seconds total
- **Visual Tests:** < 30 seconds total

### Quality Goals
- **Zero regressions:** All tests pass before merge
- **Fast feedback:** Tests run in < 1 minute
- **Easy to write:** New tests take < 5 minutes

---

## 🎯 Next Steps

1. **Immediate (Today):**
   - Fix shooter game bugs (mouse, shooting, A/D)
   - Add manual testing checklist
   - Add compile test script

2. **Short-term (This Week):**
   - Add `#[test]` support to parser
   - Create `wj test` command
   - Write 10 unit tests

3. **Medium-term (This Month):**
   - Implement headless mode
   - Create input simulation API
   - Write integration tests

4. **Long-term (Next Month):**
   - Add visual regression tests
   - Implement property-based tests
   - Full CI/CD integration

---

**Status:** 📋 **PLAN COMPLETE** - Ready for implementation!  
**Next:** Fix immediate bugs, then start Phase 1

