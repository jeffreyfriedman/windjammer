use windjammer_game::prelude::*;


struct CubeGame {
    rotation: f32,
    camera: Camera,
}

impl CubeGame {
#[inline]
fn new(&self) -> Self {
        Self { rotation: 0.0, camera: Camera::new() }
}
}

impl GameLoop for CubeGame {
#[inline]
fn init(&self) {
        println!("3D Cube Demo - Rotating cube with perspective camera")
}
#[inline]
fn update(&self, delta: f32) {
        self.rotation += delta * 45.0;
        let angle = self.rotation * 0.5;
        self.camera.position = Vec3::new(angle.cos() * 5.0, 3.0, angle.sin() * 5.0);
        self.camera.target = Vec3::ZERO;
}
#[inline]
fn render(&self, ctx: &mut RenderContext) {
        ctx.clear(Color::new(0.1, 0.1, 0.15, 1.0));
        ctx.set_camera(self.camera);
        let transform = Mat4::from_rotation_y(self.rotation.to_radians());
        ctx.draw_cube(Vec3::ZERO, Vec3::ONE, Color::new(0.3, 0.6, 0.9, 1.0), transform);
        ctx.draw_grid(10, 1.0, Color::new(0.3, 0.3, 0.3, 0.5));
        ctx.draw_text("Rotation: {rotation:.1}°", 10.0, 10.0, Color::WHITE)
}
}

fn main() {
    let game = CubeGame::new();
    windjammer_game.run(game)
}

