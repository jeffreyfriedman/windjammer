#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Keyframe {
    time: f32,
    value: f32,
}










#[inline]
fn find_keyframe_range(keyframes: &Vec<Keyframe>, target_time: f32) -> (usize, usize) {
    let before_idx = 0_i32;
    let mut after_idx = keyframes.len() - 1_usize;
    for i in 0_usize..keyframes.len() {
        if keyframes[i].time >= target_time {
            after_idx = i + 1_usize;
            break;
        }
    }
    (before_idx, after_idx)
}

fn main() {
    let mut frames: Vec<Keyframe> = Vec::new();
    frames.push(Keyframe { time: 0.0_f32, value: 0.0_f32 });
    frames.push(Keyframe { time: 1.0_f32, value: 10.0_f32 });
    frames.push(Keyframe { time: 2.0_f32, value: 20.0_f32 });
    let (before, after) = find_keyframe_range(&frames, 1.5_f32);
    println!("Range: {} to {}", before, after);
    println!("✅ Loop index type inference works!");
}

