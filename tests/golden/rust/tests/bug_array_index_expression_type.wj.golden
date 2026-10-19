#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Keyframe {
    time: f32,
}









#[derive(Debug, Clone, PartialEq, Default)]

struct AnimationClip {
    keyframes: Vec<Keyframe>,
}

impl AnimationClip {
#[inline]
fn find_keyframe_index(&self, target_time: f32) -> usize {
        let mut before_idx: usize = 0_usize;
        let mut after_idx = self.keyframes.len() - 1_usize;
        let mut i: usize = 0_usize;
        while i < self.keyframes.len() {
            if self.keyframes[i].time <= target_time && target_time <= self.keyframes[i + 1].time {
                before_idx = i;
                after_idx = i + 1_usize;
                break;
            }
            i += 1;
        }
        after_idx
}
#[inline]
fn another_test(&self, idx: usize) -> f32 {
        if idx + 1_usize < self.keyframes.len() {
            return self.keyframes[idx + 1].time;
        }
        0.0_f32
}
}

fn main() {
    let mut clip = AnimationClip { keyframes: vec![] };
    let idx = clip.find_keyframe_index(5.0_f32);
    let time = clip.another_test(idx);
    println!("Found keyframe at index: {}, time: {}", idx, time);
}

