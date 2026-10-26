"""Generate a python equivalent of a small bench kernel (fn/let/while/if/return/println, int arithmetic).
Raises Untranslatable for anything else, so a kernel the translator cannot mirror is reported, not guessed."""
import re


class Untranslatable(Exception):
    pass


_KW = ("return", "if", "while", "for", "else", "print", "def", "pass")
_INLINE_IF = re.compile(r"^if (.+?) \{ (.+?) \}(?: else \{ (.+?) \})?$")


def _expr(e):
    if "/" in e:
        raise Untranslatable("division (int vs float semantics differ)")
    if re.search(r"[{};]|\bas\b|::|\.", e):
        raise Untranslatable(f"expression {e!r}")
    return e.replace("&&", " and ").replace("||", " or ")


def _stmt(s):
    s = s.strip().rstrip(";")
    m = re.match(r"let (?:mut )?(\w+)(?:: [\w<>]+)? = (.+)$", s)
    if m:
        return f"{m.group(1)} = {_expr(m.group(2))}"
    m = re.match(r'println\("\$\{(.+)\}"\)$', s)
    if m:
        return f"print({_expr(m.group(1))})"
    m = re.match(r"return (.+)$", s)
    if m:
        return f"return {_expr(m.group(1))}"
    if re.match(r"^\w+ [-+*%]?= .+$", s):
        return _expr(s)
    raise Untranslatable(f"statement {s!r}")


def translate(src):
    out, depth, stack = [], 0, []
    lines = [l.strip() for l in src.splitlines() if l.strip() and not l.strip().startswith("//")]
    for l in lines:
        ind = "    " * depth
        m = re.match(r"fn (\w+)\(([^)]*)\)(?: -> [\w<>]+)? \{$", l)
        if m:
            args = ", ".join(a.split(":")[0].strip() for a in m.group(2).split(",") if a.strip())
            out.append(f"{ind}def {m.group(1)}({args}):")
            stack.append("fn"); depth += 1
            continue
        m = re.match(r"(while|if) (.+) \{$", l)
        if m:
            out.append(f"{ind}{m.group(1)} {_expr(m.group(2))}:")
            stack.append(m.group(1)); depth += 1
            continue
        if l == "} else {":
            depth -= 1
            out.append("    " * depth + "else:"); depth += 1
            continue
        if l == "}":
            kind = stack.pop(); depth -= 1
            if kind == "fn" and out:
                last = out[-1].strip()
                if not last.startswith(_KW) and "=" not in last.split("(")[0]:
                    out[-1] = out[-1].replace(last, "return " + last)
            continue
        m = _INLINE_IF.match(l)
        if m:
            out.append(f"{ind}if {_expr(m.group(1))}:")
            out.append(f"{ind}    {_stmt(m.group(2))}")
            if m.group(3):
                out.append(f"{ind}else:")
                out.append(f"{ind}    {_stmt(m.group(3))}")
            continue
        try:
            out.append(ind + _stmt(l))
        except Untranslatable:
            if depth and stack and stack[-1] == "fn" and re.match(r"^[\w()+\-*% <>=!,]+$", l):
                out.append(ind + l)  # trailing bare expression (becomes `return` at the closing brace)
            else:
                raise
    if depth:
        raise Untranslatable("unbalanced braces")
    out.append("main()")
    return "\n".join(out) + "\n"
