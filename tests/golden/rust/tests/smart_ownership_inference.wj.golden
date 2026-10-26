#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}
impl Vec3 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(12);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes.extend_from_slice(&self.z.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Default)]

struct Mat4 {
    m00: f32,
    m01: f32,
    m02: f32,
    m03: f32,
}
impl Mat4 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(16);
        __bytes.extend_from_slice(&self.m00.to_ne_bytes());
        __bytes.extend_from_slice(&self.m01.to_ne_bytes());
        __bytes.extend_from_slice(&self.m02.to_ne_bytes());
        __bytes.extend_from_slice(&self.m03.to_ne_bytes());
        __bytes
    }
}


impl Vec3 {
#[inline]
fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
}
#[inline]
fn dot(&self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
}
#[inline]
fn get_x(&self) -> f32 {
        self.x
}
}

impl Vec3 {
#[inline]
fn set_x(&mut self, value: f32) {
        self.x = value;
}
#[inline]
fn scale(&mut self, factor: f32) {
        self.x *= factor;
        self.y *= factor;
        self.z *= factor;
}
#[inline]
fn normalize(&mut self) {
        let len = self.length_squared();
        self.x /= len;
        self.y /= len;
        self.z /= len;
}
}

impl Mat4 {
#[inline]
fn multiply(self, other: Mat4) -> Mat4 {
        Mat4 { m00: self.m00 * other.m00, m01: self.m01 * other.m01, m02: self.m02 * other.m02, m03: self.m03 * other.m03 }
}
}

fn test_immutable_reads() {
    println!("TEST: Immutable reads should not require mut");
    let v = Vec3 { x: 3.0_f32, y: 4.0_f32, z: 0.0_f32 };
    let len_sq = v.length_squared();
    assert!(len_sq == 25.0_f32, "Length squared should be 25");
    let x = v.get_x();
    assert!(x == 3.0_f32, "X should be 3");
    let other = Vec3 { x: 1.0_f32, y: 0.0_f32, z: 0.0_f32 };
    let dot_product = v.dot(other);
    assert!(dot_product == 3.0_f32, "Dot product should be 3");
    println!("  ✅ Immutable reads work correctly!");
}

#[inline]
fn test_mutable_writes() {
    println!("TEST: Mutable writes should require mut");
    let mut v = Vec3 { x: 1.0_f32, y: 2.0_f32, z: 3.0_f32 };
    v.set_x(10.0_f32);
    assert!(v.x == 10.0_f32, "X should be updated to 10");
    v.scale(2.0_f32);
    assert!(v.x == 20.0_f32, "X should be scaled to 20");
    assert!(v.y == 4.0_f32, "Y should be scaled to 4");
    println!("  ✅ Mutable writes work correctly!");
}

#[inline]
fn test_copy_operators() {
    println!("TEST: Copy types in operators should be by value");
    let m1 = Mat4 { m00: 1.0_f32, m01: 2.0_f32, m02: 3.0_f32, m03: 4.0_f32 };
    let m2 = Mat4 { m00: 2.0_f32, m01: 3.0_f32, m02: 4.0_f32, m03: 5.0_f32 };
    let result = m1.multiply(m2);
    let _result2 = m1.multiply(m2);
    assert!(result.m00 == 2.0_f32, "Multiplication should work");
    println!("  ✅ Copy operators work correctly!");
}

fn main() {
    println!("========================================");
    println!("🧠 SMART OWNERSHIP INFERENCE TESTS");
    println!("========================================");
    test_immutable_reads();
    test_mutable_writes();
    test_copy_operators();
    println!("");
    println!("✅ All smart ownership inference tests passed!");
    println!("");
    println!("🎉 SMART INFERENCE WORKING! 🎉");
}

