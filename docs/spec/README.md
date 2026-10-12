# Windjammer language specification

Status: first edition, describing the language **as implemented and as decided by accepted ADRs** (ADR-0002, 0004, 0014, 0015; ADR-0005 and 0007 are still Proposed). Where those disagree the spec says so and the disagreement is an entry in [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md); nothing is silently invented.

Two audiences, one source:

- humans: the chapters below, every rule with a runnable example;
- LLM context: [SPEC_COMPACT.md](SPEC_COMPACT.md), one page (< 8k tokens), plus [grammar.ebnf](grammar.ebnf) for constrained decoding.

## How the spec is kept honest

| Artifact | Checked by |
|---|---|
| `grammar.ebnf` vs the real lexer/parser | docs-truth check `tools/docs_truth/spec_grammar.rs` (`cargo test --release --test docs_truth spec_grammar`): derives every alternative plus a seeded random corpus, parses each with the real parser; lexical rules must lex as one token |
| Every normative rule `**SPEC-AREA-NNN**` has an example; every example names a rule | `scripts/spec_check.sh` (also run by `tests/spec_conformance_test.rs`) |
| Examples behave as their `// expect:` oracle says | `tests/spec_conformance_test.rs` (in-process lexer, parser, interpreter) |
| Code blocks in chapters equal the test files | `rule_examples_in_chapters_match_the_test_files` |
| Intrinsic list, stdlib index complete | `intrinsic_registry_is_documented`, `stdlib_index_lists_every_std_module` |
| Known divergences cannot rot | examples marked `// xfail: Q-NNN` must fail; a pass fails the run |

```bash
scripts/spec_check.sh            # IDs, tests, matrix freshness (fast, no build)
scripts/spec_check.sh --write    # regenerate CONFORMANCE_MATRIX.md
scripts/spec_check.sh --run      # + cargo test --release --test spec_conformance, docs_truth spec_grammar
```

Example file format (`tests/spec/<area>/SPEC-XXX-NNN.wj`): header comments `// spec:`, `// mode:` (run, parse-ok, parse-error, panic), `// expect:` (stdout lines), `// expect-error:`, `// expect-exit:`, `// xfail:`. Oracles are derived by hand from the stated semantics (and cross-checked with Python/Rust arithmetic), never copied from interpreter output.

## Chapters

| Chapter | Area ID | Content |
|---|---|---|
| [lexical.md](lexical.md) | LEX | tokens, comments, literals, escapes |
| [grammar.md](grammar.md), [grammar.ebnf](grammar.ebnf) | GRAM | syntax conventions and the machine-checked EBNF |
| [types.md](types.md) | TYP | type forms, inference, conversions |
| [expressions.md](expressions.md) | EXPR | precedence, if/match/closures/ranges |
| [statements.md](statements.md) | STMT | let, loops, defer |
| [patterns.md](patterns.md) | PAT | all pattern forms |
| [items.md](items.md) | ITEM | fn, struct, enum, impl, const, alias |
| [modules.md](modules.md) | MOD | mod, use, paths, std imports |
| [traits.md](traits.md) | TRT | traits, generics, dispatch, derives |
| [ownership.md](ownership.md), [inference.md](inference.md) | OWN | surface rules; inference spec is a separate stream |
| [numeric.md](numeric.md) | NUM | int/float semantics (ADR-0015) |
| [strings.md](strings.md) | STR | strings, interpolation, format specs |
| [errors.md](errors.md) | ERR | Option, Result, `?`, panics |
| [concurrency.md](concurrency.md) | CONC | ADR-0004 surface (mostly planned) |
| [pipe.md](pipe.md) | PIPE | `\|>` |
| [collections.md](collections.md) | COLL | enumerable methods: group_by, count_by, unique, chunk_by, ... (ADR-0018) |
| [closures.md](closures.md) | CLOS | `_` shorthand closures (ADR-0018) |
| [guard-match.md](guard-match.md) | MATCH | `match { cond => .. }` (ADR-0018) |
| [doctests.md](doctests.md) | DOC | examples in doc comments run by `wj test` (ADR-0018) |
| [arguments.md](arguments.md) | ARGS | named arguments and parameter defaults (ADR-0018) |
| [trailing-blocks.md](trailing-blocks.md) | BLOCK | `f(x) { a -> .. }` trailing closures (ADR-0018) |
| [intrinsics.md](intrinsics.md) | INTR | print/assert/panic/debug/list literal |
| [attributes.md](attributes.md) | ATTR | `@derive`, `@test`, ... |
| [stdlib.md](stdlib.md) | STD | portable core and std index |
| [execution.md](execution.md) | EXEC | cross-backend guarantees, tiers |
| [CONFORMANCE.md](CONFORMANCE.md), [CONFORMANCE_MATRIX.md](CONFORMANCE_MATRIX.md), [BACKEND_STATUS.md](BACKEND_STATUS.md) | | rule to test mapping; measured status on the Rust and JavaScript backends |
| [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md) | Q-NNN | contradictions and ambiguities with recommendations |

## Rule markers

`**SPEC-AREA-NNN**` starts a normative rule. `[OPEN Q-NNN]` in a rule means the text states the recommended behaviour and the implementation differs. `[PLANNED]` means decided (ADR) but unimplemented. `(untestable: reason)` exempts a rule from needing an example.
