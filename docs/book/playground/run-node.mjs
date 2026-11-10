// Headless host of the browser playground (ADR-0034): runs one .wj file through windjammer_playground.wasm
// exactly as the page's Worker does, and behaves like `wj run --interpret FILE` (program stdout on stdout,
// diagnostics and errors on stderr, the same exit status). scripts/book/book_samples.wj --wasm uses it to hold
// the playground to the native interpreter on every sample of the book.
//
//   node docs/book/playground/run-node.mjs FILE.wj [--wasm PATH] [--fuel N] [--json]

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
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
const module = await compile(readFileSync(wasm));
const r = await run(module, readFileSync(file, "utf8"), { fuel });
if (json) {
  process.stdout.write(JSON.stringify(r) + "\n");
} else {
  process.stdout.write(r.stdout);
  process.stderr.write(r.stderr);
}
process.exitCode = r.exit_code;
