#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(16);
        __bytes.extend_from_slice(&self.r.to_ne_bytes());
        __bytes.extend_from_slice(&self.g.to_ne_bytes());
        __bytes.extend_from_slice(&self.b.to_ne_bytes());
        __bytes.extend_from_slice(&self.a.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Renderer {
    pub handle: i64,
}
impl Renderer {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.handle.to_ne_bytes());
        __bytes
    }
}


impl Renderer {
#[inline]
pub fn draw_circle(self, _x: f32, _y: f32, _radius: f32, color: Color) {
        let _ = color.r + color.g + color.b + color.a;
}
#[inline]
pub fn draw_rect(self, _x: f32, _y: f32, w: f32, h: f32, color: Color) {
        let _ = color.r * w * h;
}
}

#[inline]
pub fn test() {
    let renderer = Renderer { handle: 0_i64 };
    let red = Color { r: 1.0_f32, g: 0.0_f32, b: 0.0_f32, a: 1.0_f32 };
    renderer.draw_circle(100.0_f32, 100.0_f32, 50.0_f32, red.clone());
    renderer.draw_rect(0.0_f32, 0.0_f32, 200.0_f32, 100.0_f32, red);
}

