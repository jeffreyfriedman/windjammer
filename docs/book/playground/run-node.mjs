// Headless host of the browser playground (ADR-0034): runs one .wj file through windjammer_playground.wasm
// exactly as the page's Worker does, and behaves like `wj run --interpret FILE` (program stdout on stdout,
// diagnostics and errors on stderr, the same exit status). scripts/book/book_samples.wj --wasm uses it to hold
// the playground to the native interpreter on every sample of the book.
//
//   node docs/book/playground/run-node.mjs FILE.wj|PROJECT_DIR [--wasm PATH] [--fuel N] [--json]
//
// A directory is a program of several files: every .wj file under it (not hidden or `target` directories) is
// handed to the interpreter as an in-memory file, paths relative to the directory.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { compile, run, DEFAULTS } from "./wj-wasm.mjs";

const args = process.argv.slice(2);
let file = null;
let wasm = join(dirname(fileURLToPath(import.meta.url)), "windjammer_playground.wasm");
let fuel = DEFAULTS.fuel;
let json = false;
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--wasm") wasm = args[++i];
  else if (args[i] === "--fuel") fuel = Number(args[++i]);
  else if (args[i] === "--json") json = true;
  else file = args[i];
}
if (!file) {
  process.stderr.write("usage: node run-node.mjs FILE.wj [--wasm PATH] [--fuel N] [--json]\n");
  process.exit(2);
}
function projectFiles(dir) {
  const out = [];
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1))) {
      if (e.name.startsWith(".") || e.name === "target") continue;
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith(".wj")) out.push({ path: relative(dir, p).split("\\").join("/"), source: readFileSync(p, "utf8") });
    }
  };
  walk(dir);
  return out;
}

const input = statSync(file).isDirectory() ? projectFiles(file) : readFileSync(file, "utf8");
const module = await compile(readFileSync(wasm));
const r = await run(module, input, { fuel });
if (json) {
  process.stdout.write(JSON.stringify(r) + "\n");
} else {
  process.stdout.write(r.stdout);
  process.stderr.write(r.stderr);
}
process.exitCode = r.exit_code;
