use std::io::Read;

fn step(g: &Vec<Vec<bool>>, w: usize, h: usize) -> Vec<Vec<bool>> {
    let mut n = vec![vec![false; w]; h];
    for y in 0..h {
        for x in 0..w {
            let mut c = 0;
            for dy in [-1i32, 0, 1] {
                for dx in [-1i32, 0, 1] {
                    let (ny, nx) = ((y as i32 + dy).rem_euclid(h as i32), (x as i32 + dx).rem_euclid(w as i32));
                    if (dx, dy) != (0, 0) && g[ny as usize][nx as usize] {
                        c += 1;
                    }
                }
            }
            n[y][x] = matches!((g[y][x], c), (true, 2) | (true, 3) | (false, 3));
        }
    }
    n
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let hd: Vec<usize> = lines.next().unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
    let (w, h, gens) = (hd[0], hd[1], hd[2]);
    let mut grid: Vec<Vec<bool>> = lines.take(h).map(|l| l.chars().map(|c| c == '#').collect()).collect();
    for _ in 0..gens {
        grid = step(&grid, w, h);
    }
    let mut alive = 0;
    for row in &grid {
        println!("{}", row.iter().map(|&c| if c { '#' } else { '.' }).collect::<String>());
        alive += row.iter().filter(|&&c| c).count();
    }
    println!("alive: {}", alive);
}
