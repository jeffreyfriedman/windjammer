import * as fs from "node:fs";

const s = fs.readFileSync(0, "utf8");
let pos = 0;
const nextLine = (): string => {
  const i = s.indexOf("\n", pos);
  const l = i < 0 ? s.slice(pos) : s.slice(pos, i);
  pos = i < 0 ? s.length : i + 1;
  return l;
};
const k = parseInt(nextLine(), 10);
const vars = new Map<string, string>();
for (let i = 0; i < k; i++) {
  const l = nextLine();
  const e = l.indexOf("=");
  vars.set(l.slice(0, e), l.slice(e + 1));
}
const tpl = s.slice(pos);
const res = tpl.replace(/\{\{ *([A-Za-z0-9_]+) *\}\}/g, (_m, name: string) => {
  const v = vars.get(name);
  return v === undefined ? `<missing:${name}>` : v;
});
fs.writeSync(1, res);
