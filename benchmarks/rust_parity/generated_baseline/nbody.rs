#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
struct Body {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    m: f64,
}
impl Body {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(40);
        __bytes.extend_from_slice(&self.x.to_ne_bytes());
        __bytes.extend_from_slice(&self.y.to_ne_bytes());
        __bytes.extend_from_slice(&self.vx.to_ne_bytes());
        __bytes.extend_from_slice(&self.vy.to_ne_bytes());
        __bytes.extend_from_slice(&self.m.to_ne_bytes());
        __bytes
    }
}


#[inline]
fn step(bodies: Vec<Body>, dt: f64) -> Vec<Body> {
    let mut b: Vec<Body> = bodies;
    let n = b.len();
    for i in 0_usize..n {
        let mut ax = 0.0_f64;
        let mut ay = 0.0_f64;
        for j in 0_usize..n {
            if i != j {
                let dx = b[j].x - b[i].x;
                let dy = b[j].y - b[i].y;
                let d2 = dx * dx + dy * dy + 0.01_f64;
                let inv = 1.0_f64 / (d2 * d2.sqrt());
                ax += dx * b[j].m * inv;
                ay += dy * b[j].m * inv;
            }
        }
        b[i].vx += ax * dt;
        b[i].vy += ay * dt;
    }
    for i in 0_usize..n {
        b[i].x += b[i].vx * dt;
        b[i].y += b[i].vy * dt;
    }
    b
}

fn main() {
    let mut bodies = Vec::new();
    for i in 0_i32..40_i32 {
        let f = i as f64;
        bodies.push(Body { x: f * 1.5_f64, y: f * 0.7_f64 - 10.0_f64, vx: 0.0_f64, vy: 0.0_f64, m: 1.0_f64 + (i % 3_i32) as f64 });
    }
    for _s in 0_i32..2000_i32 {
        bodies = step(bodies, 0.001_f64);
    }
    let mut e: f64 = 0.0_f64;
    for i in 0_usize..bodies.len() {
        e += bodies[(i as usize)].x + bodies[(i as usize)].y;
    }
    println!("{}", e);
}

