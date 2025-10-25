//! Time management and delta time

use std::time::{Duration, Instant};

/// Time tracking for game loop
pub struct Time {
    start: Instant,
    last_update: Instant,
    delta: Duration,
    elapsed: Duration,
    frame_count: u64,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_update: now,
            delta: Duration::ZERO,
            elapsed: Duration::ZERO,
            frame_count: 0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta = now - self.last_update;
        self.elapsed = now - self.start;
        self.last_update = now;
        self.frame_count += 1;
    }

    pub fn delta(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    pub fn delta_duration(&self) -> Duration {
        self.delta
    }

    pub fn elapsed(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    pub fn elapsed_duration(&self) -> Duration {
        self.elapsed
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn fps(&self) -> f32 {
        if self.delta.as_secs_f32() > 0.0 {
            1.0 / self.delta.as_secs_f32()
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
    use std::thread;

    #[test]
    fn test_time_creation() {
        let time = Time::new();
        assert_eq!(time.frame_count(), 0);
        assert_eq!(time.elapsed(), 0.0);
    }

    #[test]
    fn test_time_update() {
        let mut time = Time::new();
        thread::sleep(Duration::from_millis(10));
        time.update();
        assert!(time.delta() > 0.0);
        assert_eq!(time.frame_count(), 1);
    }
}
