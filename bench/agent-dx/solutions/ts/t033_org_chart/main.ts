import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.trim()).filter((l) => l !== "");
const n = parseInt(lines[0], 10);
const emp = new Map<string, { mgr: string; sal: string }>();
for (let i = 1; i <= n; i++) {
  const [name, mgr, sal] = lines[i].split(/\s+/);
  emp.set(name, { mgr, sal });
}
const out: string[] = [];
for (const q of lines.slice(n + 1)) {
  const p = emp.get(q);
  if (!p) {
    out.push(`none: unknown ${q}`);
    continue;
  }
  if (p.mgr === "-") {
    out.push(`none: no manager ${q}`);
    continue;
  }
  const m = emp.get(p.mgr);
  if (!m) {
    out.push(`none: unknown ${p.mgr}`);
    continue;
  }
  if (m.mgr === "-") {
    out.push(`none: no skip-level ${p.mgr}`);
    continue;
  }
  const mm = emp.get(m.mgr);
  out.push(mm ? mm.sal : `none: unknown ${m.mgr}`);
}
if (out.length > 0) console.log(out.join("\n"));
