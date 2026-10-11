# agent-dx report

## Toolchains

- wj-interp: OK wj 0.50.0 
- wj: OK wj 0.50.0 + cargo (wj built from the scrub-compiler worktree release binary, 11:46 Oct 11) 
- rust: OK rustc 1.100.0-nightly (5db7f4be8 2026-09-01) 
- go: OK go version go1.26.1 darwin/arm64 
- python: OK Python 3.13.5 
- ts: OK node v24.17.0; typecheck via deno 

## Correctness matrix (reference solutions vs hidden vectors)

P = all hidden vectors pass, F = runs but wrong, B = does not build/run (compile or interpreter error), - = no solution, s = skipped

| task | tier | wj-interp | wj | rust | go | python | ts |
|---|---|---|---|---|---|---|---|
| t001_gcd_lcm | pure-functions | P | P | P | P | P | P |
| t002_fizzbuzz_custom | pure-functions | P | P | P | P | P | P |
| t003_prime_counts | pure-functions | P | P | P | P | P | P |
| t004_collatz | pure-functions | P | P | P | P | P | P |
| t005_roman | pure-functions | P | P | P | P | P | P |
| t006_base_convert | pure-functions | P | P | P | P | P | P |
| t007_digit_stats | pure-functions | P | P | P | P | P | P |
| t008_fib_mod | pure-functions | P | P | P | P | P | P |
| t009_rpn_eval | data-structures | P | P | P | P | P | P |
| t010_ring_buffer | data-structures | P | P | P | P | P | P |
| t011_lru_cache | data-structures | P | P | P | P | P | P |
| t012_min_heap | data-structures | P | P | P | P | P | P |
| t013_bst | data-structures | P | P | P | P | P | P |
| t014_union_find | data-structures | P | P | P | P | P | P |
| t015_linked_list | data-structures | P | P | P | P | P | P |
| t016_matrix_mult | data-structures | P | P | P | P | P | P |
| t017_interval_merge | data-structures | P | P | P | P | P | P |
| t018_word_freq | strings-parsing | P | P | P | P | P | P |
| t019_caesar | strings-parsing | P | P | P | P | P | P |
| t020_rle | strings-parsing | P | P | P | P | P | P |
| t021_anagram_groups | strings-parsing | P | P | P | P | P | P |
| t022_csv_parse | strings-parsing | P | P | P | P | P | P |
| t023_expr_eval | strings-parsing | P | P | P | P | P | P |
| t024_bracket_match | strings-parsing | P | P | P | P | P | P |
| t025_template_render | strings-parsing | P | P | P | P | P | P |
| t026_palindromes | strings-parsing | P | P | P | P | P | P |
| t027_ini_parser | strings-parsing | P | P | P | P | P | P |
| t028_safe_div_chain | error-handling | P | P | P | P | P | P |
| t029_parse_report | error-handling | P | P | P | P | P | P |
| t030_bank_ledger | error-handling | P | P | P | P | P | P |
| t031_config_validate | error-handling | P | P | P | P | P | P |
| t032_checked_arith | error-handling | P | P | P | P | P | P |
| t033_org_chart | error-handling | P | P | P | P | P | P |
| t034_shapes_trait | traits-generics | P | P | P | P | P | P |
| t035_generic_minmax_stack | traits-generics | P | P | P | P | P | P |
| t036_multikey_sort | traits-generics | P | P | P | P | P | P |
| t037_animal_traits | traits-generics | P | P | P | P | P | P |
| t038_vec2_ops | traits-generics | P | P | P | P | P | P |
| t039_custom_iterators | traits-generics | P | P | P | P | P | P |
| t040_memoizer | traits-generics | P | P | P | P | P | P |
| t041_expr_tree | traits-generics | P | P | P | P | P | P |
| t042_parallel_sum | concurrency | F | P | P | P | P | P |
| t043_channel_pipeline | concurrency | F | B | P | P | P | P |
| t044_shared_histogram | concurrency | F | P | P | P | P | P |
| t045_parallel_divisors | concurrency | F | P | P | P | P | P |
| t046_producer_consumer | concurrency | F | P | P | P | P | P |
| t047_task_scheduler | concurrency | P | P | P | P | P | P |
| t048_wc_clone | file-cli | P | P | P | P | P | P |
| t049_grep_clone | file-cli | P | P | P | P | P | P |
| t050_cat_number | file-cli | P | P | P | P | P | P |
| t051_file_transform | file-cli | P | P | P | P | P | P |
| t052_csv_columns | file-cli | P | P | P | P | P | P |
| t053_cli_flags | file-cli | P | P | P | P | P | P |
| t054_dir_tree | file-cli | F | P | P | P | P | P |
| t055_log_summary | file-cli | P | P | P | P | P | P |
| t056_todo_manager | stateful-apps | P | P | P | P | P | P |
| t057_inventory | stateful-apps | P | P | P | P | P | P |
| t058_vending_machine | stateful-apps | P | P | P | P | P | P |
| t059_game_of_life | stateful-apps | P | P | P | P | P | P |
| t060_kv_transactions | stateful-apps | P | P | P | P | P | P |

### Pass rate by language and tier

| tier | wj-interp | wj | rust | go | python | ts |
|---|---|---|---|---|---|---|
| concurrency | 1/6 | 5/6 | 6/6 | 6/6 | 6/6 | 6/6 |
| data-structures | 9/9 | 9/9 | 9/9 | 9/9 | 9/9 | 9/9 |
| error-handling | 6/6 | 6/6 | 6/6 | 6/6 | 6/6 | 6/6 |
| file-cli | 7/8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 |
| pure-functions | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 |
| stateful-apps | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 |
| strings-parsing | 10/10 | 10/10 | 10/10 | 10/10 | 10/10 | 10/10 |
| traits-generics | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 |
| **all** | **54/60** | **59/60** | **60/60** | **60/60** | **60/60** | **60/60** |

## wj-interp: tasks that do not pass

| task | status | passed | categories | first diagnostic |
|---|---|---|---|---|
| t042_parallel_sum | fail | 0/37 | api-misuse | error: no method named `spawn` found for value of type `fn`   --> /var/folders/k8/29syjv_n2255wky6v42379lw0000 |
| t043_channel_pipeline | fail | 0/38 | unsupported | error: unsupported in the interpreter: `std::sync::mpsc::channel` is not implemented by the interpreter   -->  |
| t044_shared_histogram | fail | 0/35 | unsupported | error: unsupported in the interpreter: `std::sync::Mutex::new` is not implemented by the interpreter   --> /va |
| t045_parallel_divisors | fail | 0/34 | api-misuse | error: no method named `spawn` found for value of type `fn`   --> /var/folders/k8/29syjv_n2255wky6v42379lw0000 |
| t046_producer_consumer | fail | 0/35 | unsupported | error: unsupported in the interpreter: `std::sync::mpsc::sync_channel` is not implemented by the interpreter   |
| t054_dir_tree | fail | 0/34 | unsupported | mplemented by the interpreter   --> /var/folders/k8/29syjv_n2255wky6v42379lw0000gn/T/agentdx-ws-8f29bg45/main. |

## wj: tasks that do not pass

| task | status | passed | categories | first diagnostic |
|---|---|---|---|---|
| t043_channel_pipeline | build_error | 0/38 | toolchain-bug | `std::sync::mpsc::Receiver<i64>` cannot be shared between threads safely |

## Agent runs (from recorded transcripts)

Transcripts under `transcripts/scripted/` come from scripted agents (reference solutions or canned edits) and only exercise the harness; they are not LLM results.

| lang | trials | pass@1 | within 2 iters | within 3 iters | mean iters to green | mean fix attempts | tokens in/out | wall s |
|---|---|---|---|---|---|---|---|---|
| wj-interp | 3 | 1.0 | 1.0 | 1.0 | 1.0 | None | n/a | 0.5 |
| rust | 3 | 1.0 | 1.0 | 1.0 | 1.0 | None | n/a | 0.3 |
| go | 3 | 1.0 | 1.0 | 1.0 | 1.0 | None | n/a | 0.2 |
| python | 4 | 0.75 | 1.0 | 1.0 | 1.25 | 1.0 | 3000/300 | 0.7 |
| ts | 3 | 1.0 | 1.0 | 1.0 | 1.0 | None | n/a | 0.4 |

### Diagnostic categories encountered

| lang | syntax |
|---|---|
| wj-interp | 0 |
| rust | 0 |
| go | 0 |
| python | 1 |
| ts | 0 |

## Methodology limits

See README.md: training-data contamination (Rust/Go/Python/TS are heavily represented in LLM training data, Windjammer is not), prompt variance, small trial counts, task-suite selection bias, and in-repo oracle visibility.

