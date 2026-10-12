#!/usr/bin/env bash
# Measure how many diagnostics-corpus cases leak rustc errors, and how many leak untranslated.
#   scripts/diagnostics-leak-report.sh [path/to/wj] [cases-dir]
# For every case: `wj check --json` (front end), then for cases the front end accepts, build the
# generated crate with cargo and compare raw rustc errors with the mapped `wj build --json` output.
#   leak        = a rustc error reached the user (front end did not catch the case)
#   unmapped    = a leaked rustc error that came out as WJ0099 or without a .wj location
#   baseline    = leaked errors whose rustc code the pre-change registry had no WJ code for
set -uo pipefail
WJ="${1:-wj}"
CASES="${2:-$(dirname "$0")/../tests/diagnostics/cases}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/wj-leak-target}"
OLD_MAPPED='E0425 E0308 E0384 E0596 E0412 E0433 E0382 E0502 E0503 E0063 E0004'
python3 - "$WJ" "$CASES" "$OLD_MAPPED" <<'PY'
import json, os, subprocess, sys, tempfile, glob
wj, cases, old = sys.argv[1], sys.argv[2], set(sys.argv[3].split())
tot = dict(cases=0, caught_front=0, leaked=0, rustc_errors=0, unmapped=0, baseline_unmapped=0, wrong_code=0)
rows = []
for f in sorted(glob.glob(os.path.join(cases, '*.wj'))):
    name = os.path.basename(f)[:-3]
    expect = open(f).readline().strip().replace('// expect: ', '')
    tot['cases'] += 1
    chk = subprocess.run([wj, 'check', '--json', f], capture_output=True, text=True)
    try: rep = json.loads(chk.stdout)
    except Exception: rep = {'diagnostics': [], 'success': False}
    front = [d for d in rep['diagnostics'] if d['severity'] == 'error']
    if front or expect == 'none':
        tot['caught_front'] += bool(front)
        rows.append((name, 'front-end', ','.join(d['code'] for d in front) or '-', 0, 0))
        continue
    out = tempfile.mkdtemp(prefix='wjleak-')
    subprocess.run([wj, 'build', f, '-o', out, '--no-cargo'], capture_output=True, text=True)
    raw = subprocess.run(['cargo', 'build', '--message-format=json'], cwd=out, capture_output=True, text=True)
    rcodes = []
    for line in raw.stdout.splitlines():
        try: m = json.loads(line)
        except Exception: continue
        msg = m.get('message')
        if m.get('reason') == 'compiler-message' and msg and msg.get('level') == 'error' and msg.get('code'):
            rcodes.append(msg['code']['code'])
    if not rcodes:
        rows.append((name, 'builds', '-', 0, 0)); continue
    tot['leaked'] += 1; tot['rustc_errors'] += len(rcodes)
    base = sum(1 for c in rcodes if c not in old)
    tot['baseline_unmapped'] += base
    js = subprocess.run([wj, 'build', f, '-o', out, '--json', '--check'], capture_output=True, text=True)
    try: ds = json.loads(js.stdout)['diagnostics']
    except Exception: ds = []
    errs = [d for d in ds if d['severity'] == 'error']
    unm = [d for d in errs if d['code'] == 'WJ0099' or not (d.get('primary_span') or {}).get('file', '').endswith('.wj')]
    tot['unmapped'] += len(unm) if errs else len(rcodes)
    want = expect.replace('rustc:', '')
    if want not in [d['code'] for d in errs]: tot['wrong_code'] += 1
    rows.append((name, 'rustc', ','.join(sorted(set(rcodes))), len(unm) if errs else len(rcodes), base))
w = max(len(r[0]) for r in rows)
print(f"{'case'.ljust(w)}  stage      rustc/front codes        unmapped  baseline-unmapped")
for r in rows: print(f"{r[0].ljust(w)}  {r[1]:<9}  {r[2]:<22}  {r[3]:<8}  {r[4]}")
print()
print(json.dumps(tot))
PY
