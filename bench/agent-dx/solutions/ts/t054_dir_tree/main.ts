import * as fs from "node:fs";
import * as path from "node:path";

let dirs = 0, files = 0;
const out: string[] = [];

function walk(dir: string, depth: number): void {
  const ents = fs.readdirSync(dir, { withFileTypes: true });
  ents.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  for (const e of ents) {
    const pad = "  ".repeat(depth);
    const full = path.join(dir, e.name);
    if (e.isDirectory()) {
      dirs++;
      out.push(`${pad}${e.name}/`);
      walk(full, depth + 1);
    } else {
      files++;
      out.push(`${pad}${e.name} (${fs.statSync(full).size} bytes)`);
    }
  }
}

walk(".", 0);
out.push(`${dirs} directories, ${files} files`);
console.log(out.join("\n"));
