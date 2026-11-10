// Runs Windjammer programs off the page's thread (ADR-0034). The page terminates this Worker when a run passes its
// wall-clock budget, so even a loop the fuel limit does not see cannot hang the tab.
import { compile, run } from "./wj-wasm.mjs";

let compiled = null;

function interpreter() {
  if (!compiled) {
    compiled = fetch(new URL("./windjammer_playground.wasm", import.meta.url))
      .then((r) => {
        if (!r.ok) throw new Error(`cannot load the interpreter (${r.status})`);
        return r.arrayBuffer();
      })
      .then(compile);
    compiled.catch(() => {
      compiled = null; // retry on the next run
    });
  }
  return compiled;
}

self.onmessage = async (ev) => {
  const { id, source, fuel } = ev.data;
  try {
    const module = await interpreter();
    const started = performance.now();
    const result = await run(module, source, { fuel });
    self.postMessage({ id, ok: true, result, ms: performance.now() - started });
  } catch (e) {
    self.postMessage({ id, ok: false, error: String(e && e.message ? e.message : e) });
  }
};

// warm up: fetch and compile while the reader is still reading
interpreter().then(
  () => self.postMessage({ ready: true }),
  (e) => self.postMessage({ ready: false, error: String(e && e.message ? e.message : e) }),
);
