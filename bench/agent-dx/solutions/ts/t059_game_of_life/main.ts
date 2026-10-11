import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.trim());
const [W, H, G] = lines[0].split(/\s+/).map(Number);
let grid: boolean[][] = [];
for (let y = 0; y < H; y++) grid.push(lines[1 + y].split("").map((c) => c === "#"));
for (let g = 0; g < G; g++) {
  const next: boolean[][] = [];
  for (let y = 0; y < H; y++) {
    const row: boolean[] = [];
    for (let x = 0; x < W; x++) {
      let n = 0;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          if (dx === 0 && dy === 0) continue;
          if (grid[(y + dy + H) % H][(x + dx + W) % W]) n++;
        }
      }
      row.push(grid[y][x] ? n === 2 || n === 3 : n === 3);
    }
    next.push(row);
  }
  grid = next;
}
const out = grid.map((r) => r.map((c) => (c ? "#" : ".")).join(""));
out.push(`alive: ${grid.flat().filter((c) => c).length}`);
console.log(out.join("\n"));
