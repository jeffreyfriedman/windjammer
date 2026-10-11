#!/bin/sh
# Trivial scripted agent: copies a reference solution into the workspace. Used by docs/CI smoke runs.
#   python3 -m agentdx run --langs python --tasks t001 --agent-cmd 'sh {harness}/agents/replay_example.sh {workspace} {task} {lang}'
ws="$1"; task="$2"; lang="$3"
case "$lang" in wj|wj-interp) d=wj; f=main.wj;; rust) d=rust; f=main.rs;; go) d=go; f=main.go;; ts) d=ts; f=main.ts;; *) d=python; f=main.py;; esac
cp "$(dirname "$0")/../solutions/$d/$task/$f" "$ws/$f"
