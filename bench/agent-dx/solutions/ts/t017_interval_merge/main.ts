import * as fs from "node:fs";

const tok = fs.readFileSync(0, "utf8").split(/\s+/).filter((x) => x.length > 0).map(BigInt);
const n = Number(tok[0] ?? 0n);
const iv: [bigint, bigint][] = [];
for (let i = 0; i < n; i++) iv.push([tok[1 + 2 * i], tok[2 + 2 * i]]);
iv.sort((x, y) => (x[0] < y[0] ? -1 : x[0] > y[0] ? 1 : x[1] < y[1] ? -1 : x[1] > y[1] ? 1 : 0));
const res: [bigint, bigint][] = [];
for (const [a, b] of iv) {
  const l = res[res.length - 1];
  if (l && a <= l[1]) {
    if (b > l[1]) l[1] = b;
  } else res.push([a, b]);
}
if (res.length > 0) console.log(res.map(([a, b]) => `${a} ${b}`).join("\n"));
