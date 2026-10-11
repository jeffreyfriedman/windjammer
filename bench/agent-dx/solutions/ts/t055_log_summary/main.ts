import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n");
if (lines[lines.length - 1] === "") lines.pop();
const re = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}) (DEBUG|INFO|WARN|ERROR) (\S+)(?: (.*))?$/;
const levels: Record<string, number> = { DEBUG: 0, INFO: 0, WARN: 0, ERROR: 0 };
const comps = new Map<string, number>();
const errors: string[] = [];
let first = "-", last = "-", malformed = 0;
for (const raw of lines) {
  const m = re.exec(raw.replace(/\r$/, ""));
  if (!m) {
    malformed++;
    continue;
  }
  const [, ts, lvl, comp, msg = ""] = m;
  levels[lvl]++;
  comps.set(comp, (comps.get(comp) ?? 0) + 1);
  if (first === "-" || ts < first) first = ts;
  if (last === "-" || ts > last) last = ts;
  if (lvl === "ERROR") errors.push(`  [${comp}] ${msg}`);
}
const top = [...comps.entries()].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0)).slice(0, 3);
const out = [
  `levels: DEBUG=${levels.DEBUG} INFO=${levels.INFO} WARN=${levels.WARN} ERROR=${levels.ERROR}`,
  "top components:",
  ...top.map(([c, n]) => `  ${c} ${n}`),
  `first: ${first}`,
  `last: ${last}`,
  "errors:",
  ...errors,
  `malformed: ${malformed}`,
];
console.log(out.join("\n"));
