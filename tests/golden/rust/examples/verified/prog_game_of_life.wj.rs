#[inline]
fn neighbors(g: &Vec<Vec<bool>>, r: i64, c: i64) -> i64 {
    let mut n = 0_i64;
    for dr in [-1, 0, 1] {
        for dc in [-1, 0, 1] {
            if dr == 0_i64 && dc == 0_i64 {
                continue;
            }
            let rr = r + dr;
            let cc = c + dc;
            if rr >= 0_i64 && rr < 5_i64 && cc >= 0_i64 && cc < 5_i64 && g[rr as usize][cc as usize] {
                n += 1;
            }
        }
    }
    n
}

#[inline]
fn render(g: &Vec<Vec<bool>>) -> String {
    let mut rows: Vec<String> = vec![];
    for r in 0_i32..5_i32 {
        let mut s = String::new();
        for c in 0_i32..5_i32 {
            s = s + &{
                if g[(r as usize)][(c as usize)] {
                    String::from("#")
                } else {
                    String::from(".")
                }
            };
        }
        rows.push(s);
    }
    rows.join(" / ")
}

fn main() {
    let mut g = vec![vec![false; 5_i32]; 5_i32];
    g[1][2] = true;
    g[2][2] = true;
    g[3][2] = true;
    for gen in 0_i64..3_i64 {
        println!("gen {}: {}", gen, render(&g));
        g = step(&g);
    }
}

#[inline]
fn step(g: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
    let mut out: Vec<Vec<bool>> = vec![];
    for r in 0_i32..5_i32 {
        let mut row: Vec<bool> = vec![];
        for c in 0_i32..5_i32 {
            let n = neighbors(g, (r as i64), (c as i64));
            let alive = g[(r as usize)][(c as usize)];
            row.push(n == 3_i64 || alive && n == 2_i64);
        }
        out.push(row);
    }
    out
}

