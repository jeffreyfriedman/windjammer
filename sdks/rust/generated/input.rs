/// Input manager for keyboard and mouse
pub struct Input {
}

impl Input {
    pub fn new() -> Input {
        todo!()
    }

    /// Returns true if key was just pressed this frame
    pub fn is_key_pressed(&mut self, key: Key) -> bool {
        todo!()
    }

    /// Returns true if key is currently held down
    pub fn is_key_held(&mut self, key: Key) -> bool {
        todo!()
    }

    /// Returns true if mouse button was just pressed
    pub fn is_mouse_button_pressed(&mut self, button: MouseButton) -> bool {
        todo!()
    }

    /// Returns current mouse position
    pub fn mouse_position(&mut self) -> Vec2 {
        todo!()
    }

}
