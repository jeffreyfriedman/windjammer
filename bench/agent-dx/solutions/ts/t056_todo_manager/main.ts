import * as fs from "node:fs";

type Item = { id: number; text: string; done: boolean };
let items: Item[] = [];
let nextId = 1;
const out: string[] = [];
const find = (arg: string) => items.find((i) => String(i.id) === String(Number(arg)) && /^\d+$/.test(arg));

for (const raw of fs.readFileSync(0, "utf8").split("\n")) {
  const line = raw.trim();
  if (line === "") continue;
  const sp = line.search(/\s/);
  const cmd = sp < 0 ? line : line.slice(0, sp);
  const rest = sp < 0 ? "" : line.slice(sp + 1).trim();
  switch (cmd) {
    case "add":
      if (rest === "") out.push("error: missing argument");
      else {
        items.push({ id: nextId, text: rest, done: false });
        out.push(`added ${nextId++}`);
      }
      break;
    case "done": {
      if (rest === "") {
        out.push("error: missing argument");
        break;
      }
      const it = find(rest.split(/\s+/)[0]);
      if (!it) out.push(`error: no such item ${rest.split(/\s+/)[0]}`);
      else if (it.done) out.push(`error: already done ${it.id}`);
      else {
        it.done = true;
        out.push(`done ${it.id}`);
      }
      break;
    }
    case "del": {
      if (rest === "") {
        out.push("error: missing argument");
        break;
      }
      const arg = rest.split(/\s+/)[0];
      const it = find(arg);
      if (!it) out.push(`error: no such item ${arg}`);
      else {
        items = items.filter((x) => x !== it);
        out.push(`deleted ${it.id}`);
      }
      break;
    }
    case "list": {
      const f = rest === "" ? "all" : rest.split(/\s+/)[0];
      if (f !== "all" && f !== "open" && f !== "done") {
        out.push(`error: bad filter ${f}`);
        break;
      }
      const sel = items.filter((i) => f === "all" || (f === "done") === i.done);
      if (sel.length === 0) out.push("(none)");
      for (const i of sel) out.push(`${i.id} [${i.done ? "x" : " "}] ${i.text}`);
      break;
    }
    case "clear": {
      const n = items.filter((i) => i.done).length;
      items = items.filter((i) => !i.done);
      out.push(`cleared ${n}`);
      break;
    }
    case "count": {
      const d = items.filter((i) => i.done).length;
      out.push(`open=${items.length - d} done=${d}`);
      break;
    }
    default:
      out.push(`error: unknown command ${cmd}`);
  }
}
if (out.length > 0) console.log(out.join("\n"));
