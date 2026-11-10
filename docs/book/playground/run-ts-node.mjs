// Headless host of the playground's TypeScript path (ADR-0034): the same type stripping (ts-core.mjs stripTypes) and
// the same Worker program (programSource: console capture, removed I/O, timer accounting) as the browser sandbox,
// run in a node:vm context instead of a Worker. scripts/book/book_samples.wj --ts-playground uses it to hold the
// playground to `node --experimental-strip-types` on every `ts,run` sample of the book.
//
//   node docs/book/playground/run-ts-node.mjs FILE.ts [--wall-ms N]

import { readFileSync } from "node:fs";
import vm from "node:vm";
import { programSource, stripTypes, TS_DEFAULTS } from "./ts-core.mjs";

const args = process.argv.slice(2);
let file = null;
let wallMs = TS_DEFAULTS.wallMs;
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--wall-ms") wallMs = Number(args[++i]);
  else file = args[i];
}
if (!file) {
  process.stderr.write("usage: node run-ts-node.mjs FILE.ts [--wall-ms N]\n");
  process.exit(2);
}

function exit(code) {
  process.exitCode = code;
  process.exit(code);
}

const timer = setTimeout(() => {
  process.stderr.write(`stopped after ${wallMs / 1000} s\n`);
  exit(124);
}, wallMs);

function onMessage(m) {
  if (m.k === "out") (m.stream === "stderr" ? process.stderr : process.stdout).write(m.text);
  else if (m.k === "limit") {
    process.stderr.write(`stopped: ${m.what} limit\n`);
    exit(124);
  } else if (m.k === "done") {
    clearTimeout(timer);
    exit(m.code);
  }
}

let js;
try {
  js = stripTypes(readFileSync(file, "utf8"));
} catch (e) {
  process.stderr.write(`TypeScript: ${e.message}\n`);
  exit(1);
}
const context = vm.createContext({
  postMessage: onMessage,
  setTimeout,
  clearTimeout,
  setInterval,
  clearInterval,
  queueMicrotask,
});
try {
  vm.runInContext(programSource(js), context, { filename: file, timeout: wallMs });
} catch (e) {
  process.stderr.write(`Uncaught ${e && e.stack ? e.stack : e}\n`);
  exit(e && e.code === "ERR_SCRIPT_EXECUTION_TIMEOUT" ? 124 : 1);
}
process.on("unhandledRejection", (r) => {
  process.stderr.write(`Uncaught ${r && r.stack ? r.stack : r}\n`);
  exit(1);
});
