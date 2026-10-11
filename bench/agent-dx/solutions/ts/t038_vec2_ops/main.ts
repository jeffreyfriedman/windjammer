import * as fs from "node:fs";

class Vec2 {
  readonly x: bigint;
  readonly y: bigint;
  constructor(x: bigint, y: bigint) {
    this.x = x;
    this.y = y;
  }
  add(o: Vec2): Vec2 { return new Vec2(this.x + o.x, this.y + o.y); }
  sub(o: Vec2): Vec2 { return new Vec2(this.x - o.x, this.y - o.y); }
  dot(o: Vec2): bigint { return this.x * o.x + this.y * o.y; }
  cross(o: Vec2): bigint { return this.x * o.y - this.y * o.x; }
  scale(k: bigint): Vec2 { return new Vec2(this.x * k, this.y * k); }
  eq(o: Vec2): boolean { return this.x === o.x && this.y === o.y; }
  toString(): string { return `(${this.x}, ${this.y})`; }
}

const out: string[] = [];
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p.length < 5) continue;
  const a = new Vec2(BigInt(p[0]), BigInt(p[1]));
  const b = new Vec2(BigInt(p[3]), BigInt(p[4]));
  switch (p[2]) {
    case "+": out.push(String(a.add(b))); break;
    case "-": out.push(String(a.sub(b))); break;
    case "*": out.push(String(a.dot(b))); break;
    case "x": out.push(String(a.cross(b))); break;
    case "s": out.push(String(a.scale(b.x))); break;
    case "==": out.push(String(a.eq(b))); break;
  }
}
if (out.length > 0) console.log(out.join("\n"));
