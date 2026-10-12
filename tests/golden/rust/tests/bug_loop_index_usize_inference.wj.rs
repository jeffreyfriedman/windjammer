#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Keyframe {
    time: f32,
    value: f32,
}










#[derive(Debug, Clone, PartialEq, Default)]

struct AnimationClip {
    keyframes: Vec<Keyframe>,
}

impl AnimationClip {
#[inline]
fn new() -> AnimationClip {
        AnimationClip { keyframes: Vec::new() }
}
#[inline]
fn add_keyframe(&mut self, time: f32, value: f32) {
        self.keyframes.push(Keyframe { time, value });
}
#[inline]
fn find_keyframe_index(&self, target_time: f32) -> usize {
        let mut after_idx = self.keyframes.len() - 1_usize;
        let mut i: usize = 0_usize;
        while i < self.keyframes.len() {
            if self.keyframes[i].time > target_time {
                after_idx = i + 1_usize;
                break;
            }
            i += 1;
        }
        after_idx
}
#[inline]
fn sample(&self, time: f32) -> f32 {
        if self.keyframes.is_empty() {
            return 0.0_f32;
        }
        let idx = self.find_keyframe_index(time);
        if idx < self.keyframes.len() {
            self.keyframes[idx].value
        } else {
            0.0_f32
        }
}
}

#[inline]
pub fn test_loop_index_usize_inference() {
    let mut clip = AnimationClip::new();
    clip.add_keyframe(0.0_f32, 0.0_f32);
    clip.add_keyframe(1.0_f32, 100.0_f32);
    clip.add_keyframe(2.0_f32, 50.0_f32);
    let idx = clip.find_keyframe_index(0.5_f32);
    assert_eq!(idx, 1_usize);
    let value = clip.sample(0.5_f32);
    assert!(value >= 0.0_f32, "Value should be valid");
    println!("✅ Loop index usize inference works correctly");
}

#[inline]
pub fn test_direct_loop_assignment() {
    let items = vec![10_i32, 20_i32, 30_i32, 40_i32, 50_i32];
    let mut found_idx = items.len();
    for i in 0_usize..items.len() {
        if items[i] > 25_i64 {
            found_idx = i;
            break;
        }
    }
    assert_eq!(found_idx, 2_usize);
    println!("✅ Direct loop assignment works");
}

#[inline]
pub fn test_loop_arithmetic_usize() {
    let data = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let mut next_idx = data.len();
    for i in 0_usize..data.len() - 1_usize {
        if data[i] == 3_i64 {
            next_idx = i + 1_usize;
            break;
        }
    }
    assert_eq!(next_idx, 3_usize);
    println!("✅ Loop arithmetic with usize works");
}

