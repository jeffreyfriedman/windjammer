import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.trim()).filter((l) => l !== "");
const cap = parseInt(lines[0], 10);
const m = new Map<string, string>();
const out: string[] = [];
for (const line of lines.slice(1)) {
  const p = line.split(/\s+/);
  if (p[0] === "put") {
    const k = String(BigInt(p[1]));
    m.delete(k);
    m.set(k, p[2]);
    if (m.size > cap) {
      const old = m.keys().next().value as string;
      m.delete(old);
      out.push(`evict ${old}`);
    }
  } else if (p[0] === "get") {
    const k = String(BigInt(p[1]));
    const v = m.get(k);
    if (v === undefined) out.push("-1");
    else {
      m.delete(k);
      m.set(k, v);
      out.push(v);
    }
  } else if (p[0] === "keys") {
    out.push(m.size ? [...m.keys()].reverse().join(" ") : "<empty>");
  }
}
if (out.length > 0) console.log(out.join("\n"));
