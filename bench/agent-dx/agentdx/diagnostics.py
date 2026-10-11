"""Map compiler/runtime diagnostics to a language-neutral category taxonomy.

Categories: syntax, name-resolution, type-mismatch, ownership-borrow, trait-bound,
api-misuse (unknown method/arg count), lint-style (wj leakage lints), unsupported
(feature not implemented by this toolchain), runtime-panic, timeout, wrong-output,
toolchain-bug (wj-internal failures, bad generated code), other.
"""
import re

CATEGORIES = ["syntax", "name-resolution", "type-mismatch", "ownership-borrow", "trait-bound",
              "api-misuse", "lint-style", "unsupported", "runtime-panic", "timeout",
              "wrong-output", "toolchain-bug", "other"]

_RULES = {
    "rust": [
        (r"E0382|E0499|E0502|E0505|E0506|E0507|E0515|E0596|E0597|E0373|E0384|E0716|E0503", "ownership-borrow"),
        (r"E0308|E0277.*(cannot add|cannot be compared|mismatch)|E0369|E0605|E0606|E0604", "type-mismatch"),
        (r"E0277|E0599.*bound|E0282|E0283|E0046|E0053|E0117|E0119", "trait-bound"),
        (r"E0425|E0412|E0433|E0432|E0405|E0423|E0424|E0426|E0531|E0532", "name-resolution"),
        (r"E0599|E0061|E0107|E0614|E0609|E0560|E0063|E0023|E0026|E0027", "api-misuse"),
        (r"expected .*found|unexpected token|this file contains an unclosed|unterminated|expected one of", "syntax"),
    ],
    "go": [
        (r"syntax error|unexpected|expected .*found", "syntax"),
        (r"undefined:|undeclared name|imported and not used|declared and not used", "name-resolution"),
        (r"cannot use|mismatched types|invalid operation|cannot convert|non-boolean condition", "type-mismatch"),
        (r"missing return|not enough arguments|too many arguments|has no field or method|assignment mismatch", "api-misuse"),
        (r"does not implement|missing method", "trait-bound"),
    ],
    "python": [
        (r"SyntaxError|IndentationError|TabError", "syntax"),
        (r"NameError|ModuleNotFoundError|ImportError|UnboundLocalError", "name-resolution"),
        (r"TypeError", "type-mismatch"),
        (r"AttributeError|KeyError|IndexError|ValueError", "api-misuse"),
    ],
    "ts": [
        (r"TS1\d\d\d|SyntaxError|Unexpected token", "syntax"),
        (r"TS2304|TS2552|TS2307|ReferenceError|TS2305", "name-resolution"),
        (r"TS2322|TS2345|TS2362|TS2363|TS2365|TS2367|TypeError", "type-mismatch"),
        (r"TS2339|TS2554|TS2551|TS2349", "api-misuse"),
        (r"TS2420|TS2416|TS2344", "trait-bound"),
    ],
    "wj": [
        (r"W000\d|leakage|Rust leakage", "lint-style"),
        (r"unsupported in the interpreter|not implemented|not supported", "unsupported"),
        (r"parse error|expected .*found|unexpected token|syntax", "syntax"),
        (r"unknown name|undefined|not found in scope|unresolved|cannot find|unknown function|unknown variable", "name-resolution"),
        (r"type mismatch|mismatched types|expected type|E0308", "type-mismatch"),
        (r"E0382|E0499|E0502|E0505|E0506|E0507|E0596|E0597|borrow|moved value", "ownership-borrow"),
        (r"E0277|E0599|trait bound|does not implement", "trait-bound"),
        (r"E04\d\d|E0\d\d\d\d", "toolchain-bug"),
        (r"no method|wrong number of arguments|arity", "api-misuse"),
    ],
}


def categorize(lang, text):
    """Return a category for one diagnostic message."""
    key = "wj" if lang.startswith("wj") else lang
    if re.search(r"panicked at|thread 'main' panicked|Traceback|runtime error|exit status 101", text) and key != "python":
        return "runtime-panic"
    for pat, cat in _RULES.get(key, []):
        if re.search(pat, text):
            return cat
    return "other"


def parse_rust_json(stderr):
    """rustc --error-format=json -> [{code, message, category}]"""
    import json
    out = []
    for ln in stderr.splitlines():
        ln = ln.strip()
        if not ln.startswith("{"):
            continue
        try:
            d = json.loads(ln)
        except ValueError:
            continue
        if d.get("level") != "error":
            continue
        code = (d.get("code") or {}).get("code") or ""
        msg = d.get("message", "")
        out.append({"code": code, "message": msg, "category": categorize("rust", f"{code} {msg}")})
    return out


def parse_text(lang, text, max_items=20):
    """Generic: split diagnostics out of raw text, categorize each."""
    items = []
    code_re = {"go": r"^(\S+\.go:\d+:\d+): (.*)$", "ts": r"error (TS\d+): (.*)$",
               "wj": r"^(?:error|Error)[: ]*(?:\[(\w+)\])?\s*(.*)$"}
    key = "wj" if lang.startswith("wj") else lang
    rx = re.compile(code_re[key], re.M) if key in code_re else None
    if rx:
        for m in rx.finditer(text):
            msg = m.group(0).strip()
            items.append({"code": "", "message": msg[:300], "category": categorize(lang, msg)})
            if len(items) >= max_items:
                break
    if not items and text.strip():
        first = text.strip().splitlines()
        # python: last line is the exception line
        msg = first[-1] if key == "python" else "\n".join(first[:5])
        items.append({"code": "", "message": msg[:300], "category": categorize(lang, text)})
    return items
