use wasm_bindgen::prelude::*;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

#[wasm_bindgen]
struct Universe {
    width: u32,
    height: u32,
    cells: Vec<bool>,
}

#[wasm_bindgen]
impl Universe {
pub fn new() -> Universe {
        let width = WIDTH;
        let height = HEIGHT;
        let cells = (0..width * height).map(|i| i % 2 == 0 || i % 7 == 0).collect();
        Universe { width: width, height: height, cells: cells }
}
pub fn width(&self) -> u32 {
        self.width
}
pub fn height(&self) -> u32 {
        self.height
}
pub fn get_cell(&self, row: u32, col: u32) -> bool {
        let idx = self.get_index(row, col);
        self.cells[idx]
}
pub fn tick(&mut self) {
        let mut next = self.cells.clone();
        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.get_index(row, col);
                let cell = self.cells[idx];
                let live_neighbors = self.live_neighbor_count(row, col);
                let next_cell = match (cell, live_neighbors) {
                    (true, x) if x < 2 => false,
                    (true, 2) | (true, 3) => true,
                    (true, x) if x > 3 => false,
                    (false, 3) => true,
                    (otherwise, _) => otherwise,
                };
                next[idx] = next_cell;
            }
        }
        self.cells = next;
}
pub fn toggle_cell(&mut self, row: u32, col: u32) {
        let idx = self.get_index(row, col);
        self.cells[idx] = !self.cells[idx];
}
pub fn reset(&mut self) {
        self.cells = (0..self.width * self.height).map(|_| false).collect();
}
pub fn randomize(&mut self) {
        self.cells = (0..self.width * self.height).map(|i| i % 2 == 0 || i % 7 == 0).collect();
}
}

impl Universe {
fn get_index(&self, row: u32, col: u32) -> usize {
        (row * self.width + col) as usize
}
fn live_neighbor_count(&self, row: u32, col: u32) -> u32 {
        let mut count = 0;
        for delta_row in 0..3 {
            for delta_col in 0..3 {
                if delta_row == 1 && delta_col == 1 {
                    continue;
                }
                let neighbor_row = (row + delta_row + self.height - 1) % self.height;
                let neighbor_col = (col + delta_col + self.width - 1) % self.width;
                let idx = self.get_index(neighbor_row, neighbor_col);
                if self.cells[idx] {
                    count = count + 1;
                }
            }
        }
        count
}
}

#[wasm_bindgen]
struct Stats {
    generation: u32,
    fps: f64,
    alive_cells: u32,
}

#[wasm_bindgen]
impl Stats {
pub fn new() -> Universe {
        let width = WIDTH;
        let height = HEIGHT;
        let cells = (0..width * height).map(|i| i % 2 == 0 || i % 7 == 0).collect();
        Universe { width: width, height: height, cells: cells }
}
pub fn update(&mut self, universe: &Universe, delta_time: f64) {
        self.generation = self.generation + 1;
        self.fps = 1000.0 / delta_time;
        self.alive_cells = universe.cells.iter().filter(|c| **c).count() as u32;
}
pub fn generation(&self) -> u32 {
        self.generation
}
pub fn fps(&self) -> f64 {
        self.fps
}
pub fn alive_cells(&self) -> u32 {
        self.alive_cells
}
}

fn render(universe: &Universe, ctx: &web_sys::CanvasRenderingContext2d, cell_size: u32) {
    let alive_color = "black";
    let dead_color = "white";
    ctx.begin_path();
    for row in 0..universe.height {
        for col in 0..universe.width {
            let idx = universe.get_index(row, col);
            let color = {
                if universe.cells[idx] {
                    alive_color.into()
                } else {
                    dead_color.into()
                }
            };
            ctx.set_fill_style(&color);
            ctx.fill_rect((col * cell_size) as f64, (row * cell_size) as f64, cell_size as f64, cell_size as f64);
        }
    }
    let grid_color = "#CCCCCC".into();
    ctx.set_stroke_style(&grid_color);
    ctx.begin_path();
    for i in 0..=universe.width {
        let x = (i * cell_size) as f64;
        ctx.move_to(x, 0.0);
        ctx.line_to(x, (universe.height * cell_size) as f64);
    }
    for i in 0..=universe.height {
        let y = (i * cell_size) as f64;
        ctx.move_to(0.0, y);
        ctx.line_to((universe.width * cell_size) as f64, y);
    }
    ctx.stroke()
}

