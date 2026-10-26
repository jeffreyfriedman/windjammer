class V2 {
  constructor(x, y) { this.x = x; this.y = y; }
  add(o) { return new V2(this.x + o.x, this.y + o.y); }
  dot(o) { return this.x * o.x + this.y * o.y; }
}
let acc = new V2(0, 0);
let s = 0;
for (let i = 0; i < 300000; i++) {
  const v = new V2(i % 13, i % 7);
  acc = acc.add(v);
  s += acc.dot(v) % 1000;
}
console.log(`${acc.x} ${acc.y} ${s}`);
