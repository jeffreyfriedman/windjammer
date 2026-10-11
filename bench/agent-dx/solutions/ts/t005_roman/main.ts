import * as fs from "node:fs";

const syms: [number, string][] = [
  [1000, "M"], [900, "CM"], [500, "D"], [400, "CD"], [100, "C"], [90, "XC"],
  [50, "L"], [40, "XL"], [10, "X"], [9, "IX"], [5, "V"], [4, "IV"], [1, "I"],
];

function toRoman(n: number): string {
  let s = "";
  for (const [v, r] of syms) {
    while (n >= v) {
      s += r;
      n -= v;
    }
  }
  return s;
}

function fromRoman(s: string): number {
  const val: Record<string, number> = { I: 1, V: 5, X: 10, L: 50, C: 100, D: 500, M: 1000 };
  let total = 0;
  for (let i = 0; i < s.length; i++) {
    const v = val[s[i]];
    const nx = i + 1 < s.length ? val[s[i + 1]] : 0;
    total += v < nx ? -v : v;
  }
  return total;
}

for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const line = raw.trim();
  if (line === "") continue;
  console.log(/^\d+$/.test(line) ? toRoman(parseInt(line, 10)) : String(fromRoman(line)));
}
