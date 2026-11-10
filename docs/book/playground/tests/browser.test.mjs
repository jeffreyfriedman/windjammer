// Browser tests of the book playground (ADR-0034), in headless Chrome through the real page:
//   * Windjammer: the wasm interpreter runs, refuses a type error with the compiler's diagnostic, stops a loop by fuel;
//   * TypeScript: runs; every escape route a program can try (network, storage, parent frame, code loading, eval,
//     forged messages) fails, and the trap servers see no request; a loop is stopped by the wall clock while the page
//     stays responsive; an output flood is stopped;
//   * the sandbox's CSP alone (no prelude) blocks the network, and the frame has no access to storage or the parent.
// Every assertion is a fact about the browser's behaviour, checked against servers that log every request.
//
//   cd docs/book/playground/tests && npm ci --ignore-scripts && CHROME_BIN=/path/to/chrome node browser.test.mjs
// Needs windjammer_playground.wasm next to index.html (scripts/book/playground_build.wj builds it).
// Browser-only plumbing: driving Chrome needs JavaScript (Windjammer has no browser automation).

import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";
import { serve } from "./serve.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const CHROME =
  process.env.CHROME_BIN ||
  ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"].find(existsSync);
if (!CHROME) throw new Error("no Chrome found: set CHROME_BIN");
if (!existsSync(join(root, "windjammer_playground.wasm"))) throw new Error("build windjammer_playground.wasm first");

const site = await serve(root);
const other = await serve(root); // a second origin, the usual exfiltration target
const browser = await puppeteer.launch({ executablePath: CHROME, headless: true, args: ["--no-first-run", "--no-default-browser-check"] });
const page = await browser.newPage();
const consoleErrors = [];
page.on("console", (m) => {
  if (m.type() === "error") consoleErrors.push(m.text());
});
await page.goto(site.url, { waitUntil: "load" });
await page.waitForFunction(() => document.getElementById("status").textContent === "Ready.", { timeout: 30_000 });

let passed = 0;
const failures = [];
async function test(name, f) {
  try {
    await f();
    passed++;
    console.log(`ok   ${name}`);
  } catch (e) {
    failures.push(name);
    console.log(`FAIL ${name}\n${e && e.stack ? e.stack : e}`);
  }
}

async function run(lang, source, timeout = 20_000) {
  await page.evaluate(
    (lang, source) => {
      const sel = document.getElementById("lang");
      sel.value = lang;
      document.getElementById("source").value = source;
      document.getElementById("run").click();
    },
    lang,
    source,
  );
  await page.waitForFunction(() => !document.getElementById("run").disabled, { timeout });
  return page.evaluate(() => ({
    stdout: document.getElementById("stdout").textContent,
    stderr: document.getElementById("stderr").textContent,
    status: document.getElementById("status").textContent,
  }));
}

const trapRequests = () => [...site.requests, ...other.requests].filter((r) => r.url.startsWith("/exfil") || r.method === "UPGRADE");

await test("windjammer: hello runs in the browser", async () => {
  const r = await run("wj", 'fn main() {\n    println("hello from wasm")\n}\n');
  assert.equal(r.stdout, "hello from wasm\n", JSON.stringify(r));
  assert.match(r.status, /exit 0/);
});

await test("windjammer: a type error shows the compiler's diagnostic and does not run", async () => {
  const r = await run("wj", 'fn main() {\n    let x: int = "text"\n    println("ran")\n}\n');
  assert.equal(r.stdout, "");
  assert.match(r.stderr, /WJ\d{4}/);
  assert.match(r.status, /compiler found problems/);
});

await test("windjammer: an infinite loop stops at the fuel limit", async () => {
  const r = await run("wj", "fn main() {\n    let mut i = 0\n    loop {\n        i += 1\n    }\n}\n", 30_000);
  assert.match(r.status, /exit 124/, JSON.stringify(r));
  assert.match(r.stderr, /fuel/);
});

await test("windjammer: files are denied", async () => {
  const r = await run("wj", 'use std::fs\n\nfn main() {\n    let t = fs::read_to_string("/etc/hosts")\n    println("${t.is_ok()}")\n}\n');
  assert.match(r.status, /exit 126/, JSON.stringify(r));
});

await test("typescript: types are stripped and the program runs", async () => {
  const r = await run("ts", "interface P { n: number }\nconst ps: P[] = [{ n: 1 }, { n: 2 }];\nconsole.log(`total ${ps.reduce((s, p) => s + p.n, 0)}`);\nconsole.log([1, 2], { a: 'x' });\n");
  assert.equal(r.stdout, "total 3\n[ 1, 2 ] { a: 'x' }\n", JSON.stringify(r));
  assert.match(r.status, /exit 0/);
});

const ESCAPES = (same, cross) => `
const targets: string[] = ["${same}exfil/same", "${cross}exfil/cross"];
const ws = (u: string) => u.replace("http", "ws");
function attempt(name: string, f: () => unknown): void {
  try {
    const r = f();
    if (r && typeof (r as Promise<unknown>).then === "function") {
      (r as Promise<unknown>).then(() => console.log(name + ": ESCAPED"), () => console.log(name + ": blocked"));
    } else {
      console.log(name + ": " + (r === "ok" ? "ESCAPED" : "blocked"));
    }
  } catch (_) {
    console.log(name + ": blocked");
  }
}
const g: any = globalThis;
for (const t of targets) {
  attempt("fetch " + t, () => g.fetch(t + "/fetch"));
  attempt("prototype fetch " + t, () => {
    for (let o = Object.getPrototypeOf(g); o; o = Object.getPrototypeOf(o)) {
      const f = Object.getOwnPropertyDescriptor(o, "fetch");
      if (f) return f.value.call(g, t + "/proto");
    }
    return "no";
  });
  attempt("xhr " + t, () => { const x = new g.XMLHttpRequest(); x.open("GET", t + "/xhr", false); x.send(); return "ok"; });
  attempt("websocket " + t, () => { new g.WebSocket(ws(t) + "/ws"); return "ok"; });
  attempt("eventsource " + t, () => { new g.EventSource(t + "/sse"); return "ok"; });
  attempt("importScripts " + t, () => { g.importScripts(t + "/import.js"); return "ok"; });
  attempt("dynamic import " + t, () => import(t + "/dyn.js"));
  attempt("sendBeacon " + t, () => (g.navigator.sendBeacon(t + "/beacon") ? "ok" : "no"));
  attempt("nested worker " + t, () => { new g.Worker(t + "/worker.js"); return "ok"; });
}
attempt("parent document", () => (g.parent && g.parent.document ? "ok" : "no"));
attempt("top document", () => (g.top && g.top.document ? "ok" : "no"));
attempt("window document", () => (g.document ? "ok" : "no"));
attempt("localStorage", () => { g.localStorage.setItem("k", "v"); return "ok"; });
attempt("indexedDB", () => { g.indexedDB.open("k"); return "ok"; });
attempt("caches", () => g.caches.open("k"));
attempt("eval", () => { (0, eval)("1"); return "ok"; });
attempt("Function constructor", () => { new Function("return 1")(); return "ok"; });
attempt("forged result message", () => { g.postMessage({ k: "done", code: 0 }); return "ok"; });
attempt("BroadcastChannel", () => { new g.BroadcastChannel("k").postMessage("x"); return "ok"; });
attempt("WebAssembly", () => { new g.WebAssembly.Module(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])); return "ok"; });
setTimeout(() => console.log("end"), 1500);
`;

await test("typescript: every escape route is blocked and no trap is reached", async () => {
  const r = await run("ts", ESCAPES(site.url, other.url));
  const lines = r.stdout.trim().split("\n");
  assert.ok(lines.length >= 25, "every attempt reported: " + r.stdout + r.stderr);
  assert.equal(lines.at(-1), "end", r.stdout + r.stderr);
  const escaped = lines.filter((l) => l.endsWith("ESCAPED"));
  assert.deepEqual(escaped, [], "escapes: " + escaped.join("; "));
  await new Promise((ok) => setTimeout(ok, 1500));
  assert.deepEqual(trapRequests(), [], "requests reached a trap");
});

await test("typescript: an infinite loop is stopped by the wall clock and the page stays responsive", async () => {
  const started = Date.now();
  const done = run("ts", "let i = 0;\nwhile (true) { i++; }\n", 15_000);
  await new Promise((ok) => setTimeout(ok, 1000));
  const t0 = Date.now();
  await page.evaluate(() => 1 + 1);
  assert.ok(Date.now() - t0 < 1000, "the page answered while the program looped");
  const r = await done;
  assert.match(r.status, /exit 124/, JSON.stringify(r));
  assert.ok(Date.now() - started < 12_000);
  const again = await run("ts", 'console.log("after");\n');
  assert.equal(again.stdout, "after\n", "the sandbox is usable again");
});

await test("typescript: an output flood is stopped", async () => {
  const r = await run("ts", 'for (;;) { console.log("x".repeat(1000)); }\n');
  assert.match(r.status, /exit 124/, JSON.stringify(r).slice(0, 300));
  assert.ok(r.stdout.length <= (1 << 20) + 1000);
});

const sandboxFrame = () => page.frames().find((f) => f.url().endsWith("/ts-sandbox.html"));

await test("sandbox CSP alone blocks the network from a Worker without the prelude", async () => {
  const frame = sandboxFrame();
  assert.ok(frame, "sandbox frame present");
  const results = await frame.evaluate(async (targets) => {
    const code = `
      const out = [];
      const tries = [];
      for (const t of ${JSON.stringify(targets)}) {
        tries.push(fetch(t + "exfil/csp-fetch").then(() => "fetch ESCAPED", () => "fetch blocked"));
        try { const x = new XMLHttpRequest(); x.open("GET", t + "exfil/csp-xhr", false); x.send(); out.push("xhr ESCAPED"); } catch (_) { out.push("xhr blocked"); }
        try { importScripts(t + "exfil/csp-import.js"); out.push("importScripts ESCAPED"); } catch (_) { out.push("importScripts blocked"); }
        tries.push(import(t + "exfil/csp-dyn.js").then(() => "import ESCAPED", () => "import blocked"));
        try { new WebSocket(t.replace("http", "ws") + "exfil/csp-ws"); out.push("websocket created"); } catch (_) { out.push("websocket blocked"); }
      }
      try { eval("1"); out.push("eval ESCAPED"); } catch (_) { out.push("eval blocked"); }
      Promise.all(tries).then((r) => postMessage(out.concat(r)));
    `;
    const w = new Worker(URL.createObjectURL(new Blob([code], { type: "text/javascript" })));
    return await new Promise((ok) => {
      w.onmessage = (e) => ok(e.data);
      setTimeout(() => ok(["timeout"]), 5000);
    });
  }, [site.url, other.url]);
  assert.ok(results.length > 1, JSON.stringify(results));
  assert.deepEqual(results.filter((x) => x.includes("ESCAPED")), [], JSON.stringify(results));
  await new Promise((ok) => setTimeout(ok, 1500));
  assert.deepEqual(trapRequests(), [], "requests reached a trap (a WebSocket object may be created, but must not connect)");
});

await test("sandbox frame has an opaque origin: no storage, no cookies, no parent DOM", async () => {
  const frame = sandboxFrame();
  const r = await frame.evaluate(() => {
    const t = (f) => {
      try {
        const v = f();
        return v === undefined || v === null ? "none" : "ESCAPED";
      } catch (_) {
        return "blocked";
      }
    };
    return {
      origin: self.origin,
      parent: t(() => window.parent.document),
      localStorage: t(() => window.localStorage.getItem("k")),
      cookie: t(() => document.cookie),
    };
  });
  assert.equal(r.origin, "null");
  assert.equal(r.parent, "blocked");
  assert.equal(r.localStorage, "blocked");
  assert.equal(r.cookie, "blocked");
});

await test("the page rejects result messages that do not come from its sandbox frame", async () => {
  const before = await page.evaluate(() => document.getElementById("stdout").textContent);
  await page.evaluate(() => window.postMessage({ type: "wj-ts-result", id: 999, stdout: "FORGED", stderr: "", exit_code: 0 }, "*"));
  await new Promise((ok) => setTimeout(ok, 200));
  const after = await page.evaluate(() => document.getElementById("stdout").textContent);
  assert.equal(after, before);
});

await browser.close();
await site.close();
await other.close();
const unexpected = consoleErrors.filter((e) => !/Content Security Policy|Refused to|ERR_|net::|Failed to load/.test(e));
if (unexpected.length) console.log("console errors:\n" + unexpected.join("\n"));
console.log(`\nbrowser tests: ${passed} passed, ${failures.length} failed`);
process.exit(failures.length ? 1 : 0);
