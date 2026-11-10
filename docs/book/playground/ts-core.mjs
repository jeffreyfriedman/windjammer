// TypeScript in the playground (ADR-0034): strip the types with the vendored sucrase, then run the JavaScript in a
// Worker behind a prelude that captures the console and removes every ambient I/O door. The sandbox iframe
// (ts-sandbox.html) and the headless node host (run-ts-node.mjs) both build the program with `programSource`, so
// the book's `ts,run` samples exercise exactly what a reader's Run button does.
//
// The prelude is defense in depth. The boundary is the browser's: an opaque-origin iframe (sandbox="allow-scripts",
// no allow-same-origin) whose CSP has connect-src 'none' and no 'unsafe-eval', inherited by the Worker it creates.

import { transform } from "./vendor/sucrase-3.35.0.min.mjs";

export const TS_DEFAULTS = Object.freeze({ wallMs: 5_000, outputBytes: 1 << 20 });

/** TypeScript to JavaScript: types erased, everything else left as written (no downlevelling, no module rewrite). */
export function stripTypes(source) {
  return transform(source, { transforms: ["typescript"], disableESTransforms: true }).code;
}

/** The names a program must not reach: network, storage, other contexts, code loading, host messaging. */
export const BANNED = Object.freeze([
  "fetch", "XMLHttpRequest", "WebSocket", "WebSocketStream", "EventSource", "WebTransport", "importScripts",
  "Worker", "SharedWorker", "ServiceWorker", "indexedDB", "caches", "localStorage", "sessionStorage", "cookieStore",
  "BroadcastChannel", "MessageChannel", "navigator", "location", "open", "postMessage", "Request", "Response",
  "FileReaderSync", "WebAssembly", "Atomics", "SharedArrayBuffer", "RTCPeerConnection",
]);

// Runs first inside the Worker (serialised with Function.prototype.toString onto one line, so it must not close over
// anything and uses block comments only).
function prelude(limits, banned) {
  "use strict";
  const g = globalThis;
  const send = g.postMessage.bind(g);
  const st = g.setTimeout.bind(g);
  const ct = g.clearTimeout.bind(g);
  const si = g.setInterval.bind(g);
  const ci = g.clearInterval.bind(g);
  let bytes = 0;
  let finished = false;
  const live = new Set();

  function inspect(v, depth, seen) {
    if (v === null) return "null";
    switch (typeof v) {
      case "string": return depth === 0 ? v : "'" + v.replace(/\\/g, "\\\\").replace(/'/g, "\\'") + "'";
      case "number": return Object.is(v, -0) ? "-0" : String(v);
      case "bigint": return String(v) + "n";
      case "undefined": return "undefined";
      case "boolean": return String(v);
      case "symbol": return String(v);
      case "function": return v.name ? "[Function: " + v.name + "]" : "[Function (anonymous)]";
    }
    if (seen.has(v)) return "[Circular *1]";
    if (depth > 2) return Array.isArray(v) ? "[Array]" : "[Object]";
    seen.add(v);
    try {
      if (v instanceof Error) return v.stack || String(v);
      if (Array.isArray(v)) return v.length === 0 ? "[]" : "[ " + v.map((x) => inspect(x, depth + 1, seen)).join(", ") + " ]";
      if (v instanceof Map) {
        const parts = [...v].map(([k, x]) => inspect(k, depth + 1, seen) + " => " + inspect(x, depth + 1, seen));
        return "Map(" + v.size + ")" + (parts.length ? " { " + parts.join(", ") + " }" : " {}");
      }
      if (v instanceof Set) {
        const parts = [...v].map((x) => inspect(x, depth + 1, seen));
        return "Set(" + v.size + ")" + (parts.length ? " { " + parts.join(", ") + " }" : " {}");
      }
      const keys = Object.keys(v);
      const ctor = v.constructor && v.constructor !== Object && v.constructor.name ? v.constructor.name + " " : "";
      if (keys.length === 0) return ctor + "{}";
      const key = (k) => (/^[A-Za-z_$][\w$]*$/.test(k) ? k : "'" + k + "'");
      return ctor + "{ " + keys.map((k) => key(k) + ": " + inspect(v[k], depth + 1, seen)).join(", ") + " }";
    } finally {
      seen.delete(v);
    }
  }

  function finish(code) {
    if (finished) return;
    finished = true;
    send({ k: "done", code });
  }

  function emit(stream, args) {
    if (finished) return;
    const text = args.map((a) => inspect(a, 0, new Set())).join(" ") + "\n";
    bytes += text.length;
    if (bytes > limits.outputBytes) {
      finished = true;
      send({ k: "limit", what: "output" });
      throw new Error("output limit");
    }
    send({ k: "out", stream, text });
  }

  const out = (...a) => emit("stdout", a);
  const err = (...a) => emit("stderr", a);
  const console = Object.freeze({ log: out, info: out, debug: out, error: err, warn: err, trace: err });

  function check() {
    st(() => {
      if (live.size === 0) finish(0);
    }, 0);
  }
  const timers = {
    setTimeout(fn, ms, ...a) {
      const id = st(() => {
        live.delete(id);
        try {
          if (typeof fn === "function") fn(...a);
        } finally {
          check();
        }
      }, ms);
      live.add(id);
      return id;
    },
    clearTimeout(id) {
      live.delete(id);
      ct(id);
      check();
    },
    setInterval(fn, ms, ...a) {
      const id = si(() => {
        if (typeof fn === "function") fn(...a);
      }, ms);
      live.add(id);
      return id;
    },
    clearInterval(id) {
      live.delete(id);
      ci(id);
      check();
    },
  };

  if (typeof g.addEventListener === "function") {
    const fail = (text) => {
      if (finished) return;
      send({ k: "out", stream: "stderr", text: "Uncaught " + text + "\n" });
      finish(1);
    };
    g.addEventListener("error", (e) => {
      e.preventDefault();
      fail(e.error && e.error.stack ? e.error.stack : e.message);
    });
    g.addEventListener("unhandledrejection", (e) => {
      e.preventDefault();
      const r = e.reason;
      fail(r && r.stack ? r.stack : String(r));
    });
  }

  /* Remove the doors from the global object and from every prototype it inherits them through. */
  for (let o = g; o; o = Object.getPrototypeOf(o)) {
    for (const n of banned) {
      if (Object.prototype.hasOwnProperty.call(o, n)) {
        try {
          delete o[n];
        } catch (_) {
          /* non-configurable: shadowed below */
        }
      }
    }
  }
  const lock = (n, value) => {
    try {
      Object.defineProperty(g, n, { value, writable: false, configurable: false, enumerable: false });
    } catch (_) {
      /* already locked */
    }
  };
  for (const n of banned) lock(n, undefined);
  lock("console", console);
  for (const [n, f] of Object.entries(timers)) lock(n, f);
  return check;
}

/** The whole Worker script for a JavaScript program (already type-stripped). */
export function programSource(js, limits = TS_DEFAULTS) {
  const l = { outputBytes: limits.outputBytes ?? TS_DEFAULTS.outputBytes };
  // the prelude shares line 1 with the program, so error positions name the reader's own lines
  const one = prelude.toString().replace(/\n\s*/g, " ");
  return (
    `const __wj_playground_finish__ = (${one})(${JSON.stringify(l)}, ${JSON.stringify(BANNED)}); ` +
    js +
    `\n;__wj_playground_finish__();\n`
  );
}
