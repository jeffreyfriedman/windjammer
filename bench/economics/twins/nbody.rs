// Rust twin of bench/interp/nbody.wj: 5 bodies, float arithmetic over a Vec of structs mutated in place.
#[derive(Clone, Copy)]
struct Body {
    x: f64,
    y: f64,
    z: f64,
    vx: f64,
    vy: f64,
    vz: f64,
    m: f64,
}

fn advance(bodies: &mut [Body], dt: f64) {
    let n = bodies.len();
    for i in 0..n {
        for j in i + 1..n {
            let dx = bodies[i].x - bodies[j].x;
            let dy = bodies[i].y - bodies[j].y;
            let dz = bodies[i].z - bodies[j].z;
            let d2 = dx * dx + dy * dy + dz * dz;
            let mag = dt / (d2 * d2.sqrt());
            let (mi, mj) = (bodies[i].m, bodies[j].m);
            bodies[i].vx -= dx * mj * mag;
            bodies[i].vy -= dy * mj * mag;
            bodies[i].vz -= dz * mj * mag;
            bodies[j].vx += dx * mi * mag;
            bodies[j].vy += dy * mi * mag;
            bodies[j].vz += dz * mi * mag;
        }
    }
    for b in bodies.iter_mut() {
        b.x += dt * b.vx;
        b.y += dt * b.vy;
        b.z += dt * b.vz;
    }
}

fn energy(bodies: &[Body]) -> f64 {
    let mut e = 0.0;
    for (i, b) in bodies.iter().enumerate() {
        e += 0.5 * b.m * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz);
        for o in &bodies[i + 1..] {
            let (dx, dy, dz) = (b.x - o.x, b.y - o.y, b.z - o.z);
            e -= b.m * o.m / (dx * dx + dy * dy + dz * dz).sqrt();
        }
    }
    e
}

fn body(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, m: f64) -> Body {
    Body { x, y, z, vx, vy, vz, m }
}

fn main() {
    let mut bodies = vec![
        body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 39.47),
        body(4.84, -1.16, -0.10, 0.60, 2.81, -0.02, 0.037),
        body(8.34, 4.12, -0.40, -1.01, 1.82, 0.008, 0.011),
        body(12.89, -15.11, -0.22, 1.08, 0.86, -0.01, 0.0017),
        body(15.37, -25.91, 0.17, 0.97, 0.59, -0.03, 0.002),
    ];
    let before = energy(&bodies);
    for _ in 0..20000 {
        advance(&mut bodies, 0.01);
    }
    let after = energy(&bodies);
    println!("{} {}", (before * 1_000_000.0) as i64, (after * 1_000_000.0) as i64);
}
