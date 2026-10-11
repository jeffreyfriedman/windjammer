import * as fs from "node:fs";

const acc = new Map<string, bigint>();
const out: string[] = [];

function run(p: string[]): string {
  const cmd = p[0];
  if (cmd === "balance") {
    const b = acc.get(p[1]);
    return b === undefined ? `error: unknown account ${p[1]}` : String(b);
  }
  if (cmd === "open") {
    const amt = BigInt(p[2]);
    if (amt < 0n) return "error: invalid amount";
    if (acc.has(p[1])) return "error: duplicate account";
    acc.set(p[1], amt);
    return "ok";
  }
  if (cmd === "deposit" || cmd === "withdraw") {
    const amt = BigInt(p[2]);
    if (amt <= 0n) return "error: invalid amount";
    const b = acc.get(p[1]);
    if (b === undefined) return `error: unknown account ${p[1]}`;
    if (cmd === "deposit") acc.set(p[1], b + amt);
    else {
      if (amt > b) return "error: insufficient funds";
      acc.set(p[1], b - amt);
    }
    return "ok";
  }
  if (cmd === "transfer") {
    const amt = BigInt(p[3]);
    if (amt <= 0n) return "error: invalid amount";
    const f = acc.get(p[1]), t = acc.get(p[2]);
    if (f === undefined) return `error: unknown account ${p[1]}`;
    if (t === undefined) return `error: unknown account ${p[2]}`;
    if (p[1] === p[2]) return "error: same account";
    if (amt > f) return "error: insufficient funds";
    acc.set(p[1], f - amt);
    acc.set(p[2], t + amt);
    return "ok";
  }
  return "";
}

for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const p = raw.trim().split(/\s+/);
  if (p[0] === "") continue;
  out.push(run(p));
}
for (const k of [...acc.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))) out.push(`${k} ${acc.get(k)}`);
if (out.length > 0) console.log(out.join("\n"));
