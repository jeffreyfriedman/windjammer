import * as fs from "node:fs";

interface Shape {
  name(): string;
  corners(): number;
  doubleArea(): bigint;
}

function make(p: string[]): Shape | null {
  const n = p.slice(1).map((x) => BigInt(x));
  const mk = (corners: number, area: bigint): Shape => ({
    name: () => p[0],
    corners: () => corners,
    doubleArea: () => area,
  });
  switch (p[0]) {
    case "square": return mk(4, 2n * n[0] * n[0]);
    case "rect": return mk(4, 2n * n[0] * n[1]);
    case "rtri": return mk(3, n[0] * n[1]);
    case "trap": return mk(4, (n[0] + n[1]) * n[2]);
    default: return null;
  }
}

const out: string[] = [];
let total = 0n;
let best: Shape | null = null;
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "") continue;
  const s = make(p);
  if (!s) continue;
  out.push(`${s.name()} corners=${s.corners()} area2=${s.doubleArea()}`);
  total += s.doubleArea();
  if (!best || s.doubleArea() > best.doubleArea()) best = s;
}
out.push(`total area2=${total}`);
out.push(best ? `largest=${best.name()} area2=${best.doubleArea()}` : "largest=none");
console.log(out.join("\n"));
