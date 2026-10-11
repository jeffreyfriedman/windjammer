import * as fs from "node:fs";

const D = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p.length < 3) continue;
  const from = BigInt(p[1]), to = BigInt(p[2]);
  let v = 0n;
  for (const c of p[0].toUpperCase()) v = v * from + BigInt(D.indexOf(c));
  if (v === 0n) {
    console.log("0");
    continue;
  }
  let s = "";
  while (v > 0n) {
    s = D[Number(v % to)] + s;
    v /= to;
  }
  console.log(s);
}
