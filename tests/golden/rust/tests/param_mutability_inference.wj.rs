#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Point {
    pub x: f32,
    pub y: f32,
}










#[inline]
fn move_point(p: &mut Point, dx: f32, dy: f32) {
    p.x += dx;
    p.y += dy;
}

#[inline]
fn swap_points(a: &mut Point, b: &mut Point) {
    let temp_x = a.x;
    let temp_y = a.y;
    a.x = b.x;
    a.y = b.y;
    b.x = temp_x;
    b.y = temp_y;
}

#[inline]
fn distance(a: Point, b: Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

