import * as fs from "node:fs";

const LO = -(1n << 31n), HI = (1n << 31n) - 1n;
const fits = (v: bigint) => v >= LO && v <= HI;

function pow(a: bigint, b: bigint): string {
  if (b < 0n) return "invalid";
  if (b === 0n) return "1";
  if (a === 0n || a === 1n) return String(a);
  if (a === -1n) return b % 2n === 0n ? "1" : "-1";
  let r = 1n;
  for (let i = 0n; i < b; i++) {
    r *= a;
    if (!fits(r)) return "overflow";
  }
  return String(r);
}

const lines = fs.readFileSync(0, "utf8").split("\n");
const out: string[] = [];
for (const raw of lines) {
  const p = raw.trim().split(/\s+/);
  if (p.length < 3) continue;
  const a = BigInt(p[1]), b = BigInt(p[2]);
  let res: string;
  if (p[0] === "pow") res = pow(a, b);
  else if ((p[0] === "div" || p[0] === "rem") && b === 0n) res = "divzero";
  else {
    const v = p[0] === "add" ? a + b : p[0] === "sub" ? a - b : p[0] === "mul" ? a * b : p[0] === "div" ? a / b : a % b;
    res = fits(v) ? String(v) : "overflow";
  }
  out.push(res);
}
if (out.length > 0) console.log(out.join("\n"));
