// The one bridge to the Windjammer interpreter compiled to WebAssembly (ADR-0034). The browser Worker and the
// headless node host (run-node.mjs) both use it, so CI exercises the exact code path a reader's Run button takes.
//
// The module imports nothing from JavaScript: no clock, no network, no files, no entropy. Its exports:
//   wj_alloc(len) -> ptr, wj_free(ptr, len), wj_run(ptr, len, fuel, output_bytes, seed) -> result length,
//   wj_result_ptr()/wj_result_len() -> the result JSON ({stdout, stderr, diagnostics, exit_code, steps}).
//
// Every run gets a fresh instance of the compiled module: a run starts from clean memory (the interpreter keeps
// parsed programs for the life of the process), and a trap in one run cannot poison the next.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export const DEFAULTS = Object.freeze({ fuel: 10_000_000, outputBytes: 1 << 20, seed: 0 });

const REQUIRED = ["memory", "wj_alloc", "wj_free", "wj_run", "wj_result_ptr", "wj_result_len"];

/** Compile windjammer_playground.wasm once (bytes: ArrayBuffer or typed array). */
export async function compile(bytes) {
  const module = await WebAssembly.compile(bytes);
  const names = WebAssembly.Module.exports(module).map((e) => e.name);
  for (const name of REQUIRED) {
    if (!names.includes(name)) throw new Error(`windjammer_playground.wasm does not export ${name}`);
  }
  if (WebAssembly.Module.imports(module).length > 0) {
    throw new Error("windjammer_playground.wasm must not import anything from its host");
  }
  return module;
}

/** Run one program in a fresh instance; returns {stdout, stderr, diagnostics, exit_code, steps}. */
export async function run(module, source, opts = {}) {
  // an option left undefined keeps its default (a spread would copy the undefined over it)
  const o = { ...DEFAULTS };
  for (const [k, v] of Object.entries(opts)) if (v !== undefined) o[k] = v;
  const ex = (await WebAssembly.instantiate(module, {})).exports;
  const bytes = encoder.encode(source);
  const ptr = ex.wj_alloc(bytes.length);
  new Uint8Array(ex.memory.buffer, ptr, bytes.length).set(bytes);
  let len;
  try {
    len = ex.wj_run(ptr, bytes.length, o.fuel, o.outputBytes, o.seed);
  } catch (e) {
    // a trap is an interpreter defect (a Rust panic), never the program's own panic (that is exit status 101);
    // the panic hook left its message as the result, still readable in the trapped instance's memory
    try {
      const n = ex.wj_result_len();
      if (n > 0) return JSON.parse(decoder.decode(new Uint8Array(ex.memory.buffer, ex.wj_result_ptr(), n)));
    } catch (_) {
      /* fall through */
    }
    return { stdout: "", stderr: `internal error in the playground interpreter: ${e}\n`, diagnostics: [], exit_code: 70, steps: 0 };
  }
  // read after the call: the run may have grown memory and detached the old buffer
  const out = new Uint8Array(ex.memory.buffer, ex.wj_result_ptr(), len);
  return JSON.parse(decoder.decode(out));
}
