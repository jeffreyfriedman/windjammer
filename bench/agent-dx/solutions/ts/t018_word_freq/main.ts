import * as fs from "node:fs";

const s = fs.readFileSync(0, "latin1");
const nl = s.indexOf("\n");
const k = parseInt(nl < 0 ? s : s.slice(0, nl), 10);
const text = nl < 0 ? "" : s.slice(nl + 1);
const m = new Map<string, number>();
for (const w of text.match(/[A-Za-z0-9]+/g) ?? []) {
  const l = w.toLowerCase();
  m.set(l, (m.get(l) ?? 0) + 1);
}
const arr = [...m.entries()].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
const out = arr.slice(0, Math.max(0, k)).map(([w, c]) => `${w} ${c}`);
if (out.length > 0) console.log(out.join("\n"));
