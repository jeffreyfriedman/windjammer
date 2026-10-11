import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.replace(/\r$/, ""));
const sep = lines.indexOf("---");
const ini = sep < 0 ? lines : lines.slice(0, sep);
const queries = sep < 0 ? [] : lines.slice(sep + 1);
if (queries.length > 0 && queries[queries.length - 1] === "") queries.pop();
const data = new Map<string, Map<string, string>>();
let cur = "";
const sec = (n: string) => {
  let m = data.get(n);
  if (!m) data.set(n, (m = new Map()));
  return m;
};
for (const raw of ini) {
  const l = raw.trim();
  if (l === "" || l[0] === ";" || l[0] === "#") continue;
  if (l[0] === "[" && l[l.length - 1] === "]") {
    cur = l.slice(1, -1);
    continue;
  }
  const e = l.indexOf("=");
  if (e < 0) continue;
  sec(cur).set(l.slice(0, e).trim(), l.slice(e + 1).trim());
}
const out = queries.map((q) => {
  const d = q.indexOf(".");
  const s = d < 0 ? "" : q.slice(0, d);
  const k = d < 0 ? q : q.slice(d + 1);
  return data.get(s)?.get(k) ?? "<none>";
});
if (out.length > 0) console.log(out.join("\n"));
