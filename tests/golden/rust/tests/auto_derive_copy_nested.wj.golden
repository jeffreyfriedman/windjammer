#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Vec3Copy {
    x: f32,
    y: f32,
    z: f32,
}











#[derive(Debug, Clone, Copy, PartialEq)]

struct Plane {
    normal: Vec3Copy,
    distance: f32,
}

#[inline]
fn test_copy_works() {
    let plane = Plane { normal: Vec3Copy { x: 1.0_f32, y: 0.0_f32, z: 0.0_f32 }, distance: 5.0_f32 };
    let d1 = use_plane(plane);
    let d2 = use_plane(plane);
    println!("Distances: {}, {}", d1, d2);
}

#[inline]
fn use_plane(p: Plane) -> f32 {
    p.distance
}

#[inline]
fn test_array_indexing() {
    let planes: [Plane; 3] = [Plane { normal: Vec3Copy { x: 1.0_f32, y: 0.0_f32, z: 0.0_f32 }, distance: 1.0_f32 }, Plane { normal: Vec3Copy { x: 0.0_f32, y: 1.0_f32, z: 0.0_f32 }, distance: 2.0_f32 }, Plane { normal: Vec3Copy { x: 0.0_f32, y: 0.0_f32, z: 1.0_f32 }, distance: 3.0_f32 }];
    let d1 = use_plane(planes[0]);
    let d2 = use_plane(planes[1]);
    let d3 = use_plane(planes[2]);
    println!("Array distances: {}, {}, {}", d1, d2, d3);
}

fn main() {
    test_copy_works();
    test_array_indexing();
    println!("✅ Auto-derive Copy works for nested structs!");
}

