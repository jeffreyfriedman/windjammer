import * as fs from "node:fs";

const lines = fs.readFileSync(0, "utf8").split("\n").map((l) => l.replace(/\r$/, ""));
const spec = lines[0].trim().split(",").map((s) => {
  const [f, d] = s.split(":");
  return { f, sign: d === "desc" ? -1 : 1 };
});
type R = { raw: string; name: string; age: number; city: string };
const recs: R[] = lines.slice(1).filter((l) => l.trim() !== "").map((raw) => {
  const [name, age, city] = raw.trim().split(/\s+/);
  return { raw, name, age: Number(age), city };
});
const c = (a: string | number, b: string | number) => (a < b ? -1 : a > b ? 1 : 0);
recs.sort((x, y) => {
  for (const { f, sign } of spec) {
    const r = c(x[f as "name" | "age" | "city"], y[f as "name" | "age" | "city"]);
    if (r !== 0) return r * sign;
  }
  return 0;
});
if (recs.length > 0) console.log(recs.map((r) => r.raw).join("\n"));
