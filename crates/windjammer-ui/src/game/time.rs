//! Time management for games

/// Time state for game loop
pub struct Time {
    /// Delta time (time since last frame in seconds)
    pub delta: f32,
    /// Total elapsed time since game start
    pub elapsed: f32,
    /// Current frame number
    pub frame: u64,
}

impl Time {
    pub fn new() -> Self {
        Self {
            delta: 0.0,
            elapsed: 0.0,
            frame: 0,
        }
    }

    /// Update time (called internally each frame)
    pub fn update(&mut self, delta: f32) {
        self.delta = delta;
        self.elapsed += delta;
        self.frame += 1;
    }

    /// Get frames per second
    pub fn fps(&self) -> f32 {
        if self.delta > 0.0 {
            1.0 / self.delta
        } else {
            0.0
        }
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_update() {
        let mut time = Time::new();
        time.update(0.016); // ~60 FPS

        assert_eq!(time.delta, 0.016);
        assert_eq!(time.elapsed, 0.016);
        assert_eq!(time.frame, 1);
    }

    #[test]
    fn test_time_fps() {
        let mut time = Time::new();
        time.update(0.016); // ~60 FPS

        let fps = time.fps();
        assert!((fps - 62.5).abs() < 0.1);
    }
}
