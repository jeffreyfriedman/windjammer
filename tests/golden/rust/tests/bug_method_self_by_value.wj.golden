#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Transform {
    x: f32,
    y: f32,
    rotation: f32,
}











impl Transform {
#[inline]
fn new() -> Transform {
        Transform { x: 0.0_f32, y: 0.0_f32, rotation: 0.0_f32 }
}
#[inline]
fn translate(self, dx: f32, dy: f32) -> Transform {
        Transform { x: self.x + dx, y: self.y + dy, rotation: self.rotation }
}
#[inline]
fn rotate(self, angle: f32) -> Transform {
        Transform { x: self.x, y: self.y, rotation: self.rotation + angle }
}
}

#[inline]
pub fn test_method_chain_by_value() {
    let transform = Transform::new().translate(10.0_f32, 20.0_f32).rotate(45.0_f32).translate(5.0_f32, 5.0_f32);
    assert_eq!(transform.x, 15.0_f32);
    assert_eq!(transform.y, 25.0_f32);
    assert_eq!(transform.rotation, 45.0_f32);
}

#[inline]
pub fn test_single_method_by_value() {
    let t1 = Transform::new();
    let t2 = t1.translate(100.0_f32, 200.0_f32);
    assert_eq!(t2.x, 100.0_f32);
    assert_eq!(t2.y, 200.0_f32);
}

