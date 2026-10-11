# Windjammer Documentation Index

Start with the [README](../README.md) for what exists today. Everything here was checked against the code on 2026-10-10 unless
marked "background" or "vision".

## Orientation
| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Layered design, today's pipeline, module documents |
| [ARCHITECTURE_IMPROVEMENT_PLAN.md](ARCHITECTURE_IMPROVEMENT_PLAN.md) | Ordered work packages and fitness tests |
| [known-gaps.md](known-gaps.md) | Ranked gaps between promises and code |
| [adr/README.md](adr/README.md) | Architecture decision records (all Proposed) |
| [proposals/rfcs/README.md](proposals/rfcs/README.md) | RFCs with implementation status and evidence |
| [../ROADMAP.md](../ROADMAP.md), [../CHANGELOG.md](../CHANGELOG.md) | Plan and history |
| [../AGENTS.md](../AGENTS.md), [../CONTRIBUTING.md](../CONTRIBUTING.md) | How to work in the repo |

## Language
| Document | Purpose |
|----------|---------|
| [spec/README.md](spec/README.md) | Language specification: rules with IDs, executable examples, machine-checked grammar, open questions, one-page [compact spec](spec/SPEC_COMPACT.md) |
| [windjammer.ebnf](windjammer.ebnf) | Superseded grammar (use spec/grammar.ebnf) |
| [MODULE_SYSTEM.md](MODULE_SYSTEM.md) | `use`, `mod`, `crate::`, source roots, `--module-file` |
| [language-design/ownership-inference.md](language-design/ownership-inference.md) | What ownership inference decides |
| [language-guide/self-parameter.md](language-guide/self-parameter.md) | `self` receiver inference |
| [design/traits.md](design/traits.md), [design/error-mapping.md](design/error-mapping.md), [design/multi-target-codegen.md](design/multi-target-codegen.md), [design/windjammer-ui.md](design/windjammer-ui.md) | Design notes (background; see status banners) |
| [design/ASYNC_EXECUTION_MODEL.md](design/ASYNC_EXECUTION_MODEL.md) | Superseded by WJ-CONC-01 and ADR-0004 |
| [language-design/WGSL_EXTERN_LET_DESIGN.md](language-design/WGSL_EXTERN_LET_DESIGN.md) | `extern let` in shaders |
| [BEST_PRACTICES.md](BEST_PRACTICES.md), [COOKBOOK.md](COOKBOOK.md), [API_REFERENCE.md](API_REFERENCE.md), [COMPARISON.md](COMPARISON.md) | User-facing references |
| [errors/errors.md](errors/errors.md) | Error catalogue |
| [tutorials/](tutorials/) | Tutorials (check against current syntax before relying on them) |

## Compiler internals
| Document | Purpose |
|----------|---------|
| [IR_SOLVER_CODEGEN_ARCHITECTURE.md](IR_SOLVER_CODEGEN_ARCHITECTURE.md) | IR, solver, backend consumption |
| [SALSA_ARCHITECTURE.md](SALSA_ARCHITECTURE.md) | Incremental queries in the language server |
| [STDLIB_ARCHITECTURE.md](STDLIB_ARCHITECTURE.md) | Standard library design |
| [CONCURRENCY_ARCHITECTURE.md](CONCURRENCY_ARCHITECTURE.md) | Concurrency design (see ADR-0004 for the open decision) |
| [OPTIMIZATION_ARCHITECTURE.md](OPTIMIZATION_ARCHITECTURE.md), [COMPILER_OPTIMIZATIONS.md](COMPILER_OPTIMIZATIONS.md) | Optimizer designs; the optimizer is disabled (ADR-0006) |
| [EXPORT_TARGETS.md](EXPORT_TARGETS.md), [WEB_EXPORT_STRATEGY.md](WEB_EXPORT_STRATEGY.md), [CROSS_PLATFORM_VISION.md](CROSS_PLATFORM_VISION.md), [MULTI_LANGUAGE_OPTIMIZATION.md](MULTI_LANGUAGE_OPTIMIZATION.md), [MULTI_LANGUAGE_SDK_ARCHITECTURE.md](MULTI_LANGUAGE_SDK_ARCHITECTURE.md) | Multi-target vision (vision, not status; see ADR-0007) |
| [TESTING_STRATEGY.md](TESTING_STRATEGY.md), [USER_LIBRARY_TEST_GAPS.md](USER_LIBRARY_TEST_GAPS.md) | Testing |
| [TOOLING_VISION.md](TOOLING_VISION.md) | Tooling vision (not status) |

## Release and operations
[VERSIONING.md](VERSIONING.md), [PUBLISHING.md](PUBLISHING.md), [DOWNSTREAMAPP_WJ_PIN.md](DOWNSTREAMAPP_WJ_PIN.md).

## Specs and plans
[superpowers/specs/](superpowers/specs/) and [superpowers/plans/](superpowers/plans/) hold dated design specs and implementation plans
(for example the shared cargo cache design). They are point-in-time documents.

## Archive
[archive/README.md](archive/README.md) lists superseded documents kept for history.
