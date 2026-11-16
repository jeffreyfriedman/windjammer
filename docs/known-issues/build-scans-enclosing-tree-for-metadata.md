# A single-file `wj build` scans the directory tree around the file for metadata

Found by the economics suite (docs/economics/RESULTS.md, 2026-11-16).

`wj build --no-cargo -o OUT dir/main.wj` costs about ten times more CPU when `dir` sits inside a large tree than when
the same file sits alone in a directory:

| where the file is | CPU s (user+sys) | instructions retired |
|---|---:|---:|
| `bench/agent-dx/solutions/wj/t001_gcd_lcm/main.wj` in this repository | 1.26 | 6.7 G |
| a copy in an empty temporary directory | 0.09 | 0.58 G |

(`/usr/bin/time -l`, macOS arm64, wj 37d50458ec6a, loaded machine; the ratio is the point, not the seconds.)

`sample` shows all of the time in `metadata::merge_wj_meta_signatures_from_dir_inner`, called from
`merge_reached_peers_of_file` -> `merge_wj_meta_signatures_and_copy_structs_multi` -> `sidecar_policy::merge_root`,
recursing through every subdirectory of each root it is given (the file's directory and the metadata roots above it)
to find `*.wj.meta` files.

Why it matters: build cost depends on where a file is checked out, not on the program (the location-agnostic rule), and
every single-file build in a large checkout pays it. Expected fix: a single-file build reads only the metadata of
peers it reaches (`use`/`mod`), from the cache index, without walking directories; gate: the transpile CPU of a fixed
file inside this repository within 1.5x of the same file in an empty directory.
