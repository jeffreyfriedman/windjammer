#!/usr/bin/env python3
"""Generate type-error mutants of the `// rust: ok` verified examples for the check negative gate.

Each operator makes one deterministic textual change that turns a program rustc accepts into one
rustc must reject. Output: tests/check_negative/mutants/<example>__<op>.wj (header names the change).
Usage: scripts/gen-check-mutants.py [--per-op N]   (default 14 programs per operator)
"""
import os, re, sys, glob

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
OUT = os.path.join(ROOT, 'tests/check_negative/mutants')
PER_OP = int(sys.argv[sys.argv.index('--per-op') + 1]) if '--per-op' in sys.argv else 14

def body(src):
    return '\n'.join(l for l in src.split('\n') if not l.startswith('// '))

def fn_sigs(src):
    """name -> list of param types, for top-level non-generic fns (not main)."""
    out = {}
    for m in re.finditer(r'^fn (\w+)\(([^)]*)\)', src, re.M):
        name, params = m.group(1), m.group(2).strip()
        if name == 'main' or '<' in src[m.start():m.end()]:
            continue
        ps = [p.split(':', 1)[1].strip() for p in params.split(',') if ':' in p] if params else []
        out[name] = ps
    return out

def op_let_int_gets_string(src):
    m = re.search(r'^(\s*let (?:mut )?\w+: (?:int|i32|i64|usize|f64|float) = )([^\n]+)$', src, re.M)
    return m and src[:m.start(2)] + '"oops"' + src[m.end(2):]

def op_let_string_gets_int(src):
    m = re.search(r'^(\s*let (?:mut )?\w+: string = )([^\n]+)$', src, re.M)
    return m and src[:m.start(2)] + '42' + src[m.end(2):]

def op_let_bool_gets_int(src):
    m = re.search(r'^(\s*let (?:mut )?\w+: bool = )([^\n]+)$', src, re.M)
    return m and src[:m.start(2)] + '1' + src[m.end(2):]

def op_undefined_variable(src):
    for m in re.finditer(r'^\s*let (?:mut )?(\w+)\b[^\n]*$', src, re.M):
        name = m.group(1)
        if len(name) < 2 or name == '_':
            continue
        use = re.compile(r'(?<![\w.])' + re.escape(name) + r'(?!\w)')
        for u in use.finditer(src, m.end()):
            line = src[src.rfind('\n', 0, u.start()) + 1:u.start()]
            if re.match(r'\s*(let|fn|for)\b', line) or '"' in line and '${' not in line:
                continue
            return src[:u.start()] + name + '_undefined' + src[u.end():]
    return None

def in_plain_string(src, pos):
    """True when `pos` is inside a string literal but not inside a `${..}` interpolation."""
    line = src[src.rfind('\n', 0, pos) + 1:pos]
    if line.count('"') % 2 == 0:
        return False
    tail = line[line.rfind('"'):]
    return tail.rfind('${') <= tail.rfind('}')

def call_sites(src, sigs):
    for m in re.finditer(r'(?<![\w.:])(\w+)\(', src):
        name = m.group(1)
        if name not in sigs or in_plain_string(src, m.start()):
            continue
        line_start = src.rfind('\n', 0, m.start()) + 1
        if src[line_start:m.start()].lstrip().startswith('fn '):
            continue
        depth, i = 1, m.end()
        while i < len(src) and depth:
            depth += {'(': 1, ')': -1}.get(src[i], 0)
            i += 1
        yield name, m.end(), i - 1

def op_drop_argument(src):
    sigs = fn_sigs(src)
    for name, a, b in call_sites(src, sigs):
        args = src[a:b]
        if len(sigs[name]) >= 1 and args.strip() and '(' not in args:
            parts = args.split(',')
            new = ','.join(parts[:-1]) if len(parts) > 1 else ''
            return src[:a] + new + src[b:]
    return None

def op_extra_argument(src):
    sigs = fn_sigs(src)
    for name, a, b in call_sites(src, sigs):
        args = src[a:b]
        if '(' in args:
            continue
        new = (args + ', 0') if args.strip() else '0'
        return src[:a] + new + src[b:]
    return None

def op_argument_wrong_type(src):
    sigs = fn_sigs(src)
    for name, a, b in call_sites(src, sigs):
        if sigs[name] and sigs[name][0] in ('int', 'i32', 'i64', 'float', 'f64', 'bool'):
            args = src[a:b]
            if '(' in args or not args.strip():
                continue
            parts = args.split(',')
            parts[0] = '"oops"'
            return src[:a] + ','.join(parts) + src[b:]
    return None

def op_return_wrong_type(src):
    m = re.search(r'^(fn \w+\([^)]*\) -> (?:int|i64|i32|bool|float|f64) \{\n)', src, re.M)
    return m and src[:m.end()] + '    return "oops"\n' + src[m.end():]

def op_unknown_field(src):
    structs = {}
    for m in re.finditer(r'^struct (\w+) \{([^}]*)\}', src, re.M):
        structs[m.group(1)] = [f.split(':')[0].strip() for f in m.group(2).split(',') if ':' in f]
    for s, fields in structs.items():
        for f in fields:
            m = re.search(r'self\.' + re.escape(f) + r'(?!\w)', src)
            if m:
                return src[:m.start()] + 'self.' + f + 'x' + src[m.end():]
    return None

def op_undefined_function(src):
    sigs = fn_sigs(src)
    for name, a, b in call_sites(src, sigs):
        start = a - len(name) - 1
        return src[:start] + name + '_missing(' + src[a:]
    return None

def op_int_annotated_string(src):
    m = re.search(r'^(\s*let (?:mut )?\w+)( = )(-?\d+)\s*$', src, re.M)
    return m and src[:m.end(1)] + ': string' + src[m.end(1):]

def op_string_annotated_int(src):
    m = re.search(r'^(\s*let (?:mut )?\w+)( = )"[^"\n]*"\s*$', src, re.M)
    return m and src[:m.end(1)] + ': int' + src[m.end(1):]

def op_unknown_method(src):
    m = re.search(r'\.len\(\)', src)
    return m and src[:m.start()] + '.length_of()' + src[m.end():]

OPS = [op_int_annotated_string, op_string_annotated_int, op_unknown_method,

    op_let_int_gets_string, op_let_string_gets_int, op_let_bool_gets_int, op_undefined_variable,
    op_drop_argument, op_extra_argument, op_argument_wrong_type, op_return_wrong_type,
    op_unknown_field, op_undefined_function,
]

def main():
    os.makedirs(OUT, exist_ok=True)
    for f in glob.glob(os.path.join(OUT, '*.wj')):
        os.remove(f)
    examples = sorted(p for p in glob.glob(os.path.join(ROOT, 'examples/verified/*.wj'))
                      if '// rust: ok' in open(p).read())
    total = 0
    for op in OPS:
        n = 0
        for p in examples:
            if n >= PER_OP:
                break
            src = body(open(p).read())
            mutant = op(src)
            if not mutant or mutant == src:
                continue
            name = os.path.basename(p)[:-3] + '__' + op.__name__[3:]
            with open(os.path.join(OUT, name + '.wj'), 'w') as fh:
                fh.write('// mutant: %s of examples/verified/%s\n' % (op.__name__[3:], os.path.basename(p)))
                fh.write(mutant)
            n += 1
        total += n
        print('%-28s %d' % (op.__name__, n))
    print('total', total)

main()
