struct Body {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    m: f64,
}

fn step(b: &mut [Body], dt: f64) {
    let n = b.len();
    for i in 0..n {
        let (mut ax, mut ay) = (0.0, 0.0);
        for j in 0..n {
            if i != j {
                let dx = b[j].x - b[i].x;
                let dy = b[j].y - b[i].y;
                let d2 = dx * dx + dy * dy + 0.01;
                let inv = 1.0 / (d2 * d2.sqrt());
                ax += dx * b[j].m * inv;
                ay += dy * b[j].m * inv;
            }
        }
        b[i].vx += ax * dt;
        b[i].vy += ay * dt;
    }
    for body in b.iter_mut() {
        body.x += body.vx * dt;
        body.y += body.vy * dt;
    }
}

fn main() {
    let mut bodies: Vec<Body> = (0..40)
        .map(|i| {
            let f = i as f64;
            Body { x: f * 1.5, y: f * 0.7 - 10.0, vx: 0.0, vy: 0.0, m: 1.0 + (i % 3) as f64 }
        })
        .collect();
    for _ in 0..2000 {
        step(&mut bodies, 0.001);
    }
    let e: f64 = bodies.iter().fold(0.0, |e, b| e + (b.x + b.y));
    println!("{}", e);
}
