#!/usr/bin/env python3
"""
Assign each top-level tests/*.rs to a subdirectory (Phase 18).
Run from windjammer repo root: python3 scripts/reorganize_tests_phase18.py [--apply]
"""
from __future__ import annotations

import argparse
import collections
import os
import subprocess

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def categorize(name: str) -> str:
    n = name.lower()
    if name == "integration_test_helpers_self_test.rs":
        return "integration"

    if n.startswith("linter"):
        return "linter"

    if (
        n.startswith("conformance_")
        or n.startswith("cross_backend_")
        or name == "integration_backend_conformance_test.rs"
    ):
        return "conformance"

    if n.startswith("interpreter_") or name == "interpreter_test.rs":
        return "interpreter"

    if n.startswith("go_backend") or n.startswith("go_enum"):
        return "codegen/go"

    if n.startswith("js_backend"):
        return "codegen/js"

    if n.startswith("codegen_") or name == "mod_wj_codegen_test.rs":
        return "codegen/rust"

    if n.startswith("bug_") or (n.startswith("same_name_") and "bug" in n):
        return "regression"

    if n.startswith("parser_") or n.startswith("test_wjsl_"):
        return "parser"
    if n.startswith("wjsl_"):
        return "parser"
    if name in ("shader_wjsl_test.rs", "shader_file_detection_test.rs"):
        return "parser"

    if (
        n.startswith("integration_")
        or name == "stress_test_large_codebase.rs"
        or n.startswith("build_system_")
        or n.startswith("test_framework_")
        or n.startswith("project_paths_")
        or n.startswith("platform_specific_")
        or name == "compiler_tests.rs"
        or n.endswith("_integration_test.rs")
        or n.startswith("ffi_")
        or n.startswith("cross_crate_")
        or n.startswith("cross_module_")
        or n.startswith("cross_file_")
        or n.startswith("nested_module_")
        or n.startswith("module_system_")
        or n.startswith("module_tree_")
        or n.startswith("module_reexport_")
        or n.startswith("module_imports")
        or n.startswith("module_existence_")
        or n.startswith("module_declaration_")
        or n.startswith("super_import_")
        or n.startswith("mod_rs_")
        or n.startswith("lib_vs_mod_")
        or n.startswith("lib_rs_subdirectory_")
        or n.startswith("no_inline_")
        or n.startswith("out_of_scope_")
        or n.startswith("crate_imports_")
        or n.startswith("crate_internal_")
        or n.startswith("error_system")
        or name == "svo_bfs_layout_test.rs"
        or name == "vec_copy_element_deref_test.rs"
        or name == "trait_impl_multi_file_test.rs"
        or name == "trait_method_signature_edge_cases_test.rs"
        or n.startswith("dogfooding_")
        or n.startswith("wasm_")
        or n.startswith("extern_")
        or n.startswith("passthrough_qualified_")
    ):
        return "integration"

    if name == "generic_owned_param_integration_test.rs":
        return "integration"

    # Default: analyzer + type system + codegen-adjacent single-file compiler tests.
    return "analyzer"


def gather_top_level_rs() -> list[str]:
    tests_dir = os.path.join(REPO_ROOT, "tests")
    out = []
    for fn in sorted(os.listdir(tests_dir)):
        if not fn.endswith(".rs"):
            continue
        path = os.path.join(tests_dir, fn)
        if os.path.isfile(path):
            out.append(fn)
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true", help="Run git mkdir -r and git mv")
    args = ap.parse_args()

    os.chdir(REPO_ROOT)
    files = gather_top_level_rs()
    buckets: dict[str, list[str]] = collections.defaultdict(list)
    for fn in files:
        cat = categorize(fn)
        buckets[cat].append(fn)

    for cat in sorted(buckets.keys()):
        print(f"{cat}: {len(buckets[cat])}")

    cargo_lines: list[str] = []
    for cat in sorted(buckets.keys()):
        subdir = os.path.join("tests", cat.replace("/", os.sep))
        if args.apply:
            os.makedirs(subdir, exist_ok=True)
        for fn in sorted(buckets[cat]):
            rel = os.path.join("tests", cat, fn).replace(os.sep, "/")
            stem = fn[:-3]
            cargo_lines.append("[[test]]")
            cargo_lines.append(f'name = "{stem}"')
            cargo_lines.append(f'path = "{rel}"')
            cargo_lines.append("")
            if args.apply:
                src = os.path.join("tests", fn)
                dst = os.path.join(subdir, fn)
                subprocess.run(["git", "mv", src, dst], check=True)

    outp = os.path.join(REPO_ROOT, "scripts", "generated_test_targets.toml")
    body = "\n".join(cargo_lines)
    os.makedirs(os.path.dirname(outp), exist_ok=True)
    if args.apply:
        with open(outp, "w", encoding="utf-8") as f:
            f.write(body)
        print(f"Wrote {outp} (merge [[test]] blocks after [dev-dependencies]; keep autotests=false under [package])")


if __name__ == "__main__":
    main()
