#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Renderer {
    pub handle: i64,
}

impl Renderer {
#[inline]
pub fn draw_circle(&self, _x: f32, _y: f32, _radius: f32, color: Color) {
        let _ = color.r + color.g + color.b + color.a;
}
#[inline]
pub fn draw_rect(&self, _x: f32, _y: f32, w: f32, h: f32, color: Color) {
        let _ = color.r * w * h;
}
}

#[inline]
pub fn test() {
    let renderer = Renderer { handle: 0 };
    let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    renderer.draw_circle(100.0, 100.0, 50.0, red);
    renderer.draw_rect(0.0, 0.0, 200.0, 100.0, red);
}

