Implement a todo-list command interpreter. Ids start at 1 and increase by one for every `add` (ids are never reused, even after deletion). Commands, one per line:

- `add TEXT...` : add an open item with the rest of the line as its text (may contain spaces). Print `added <id>`.
- `done ID` : mark done; print `done <id>`, or `error: no such item <id>`; marking an already-done item prints `error: already done <id>`.
- `del ID` : delete; print `deleted <id>` or `error: no such item <id>`.
- `list [all|open|done]` : (default `all`) print each matching item in id order as `<id> [ ] <text>` (open) or `<id> [x] <text>` (done); print `(none)` if nothing matches. An unknown filter prints `error: bad filter <word>`.
- `clear` : delete all done items; print `cleared <n>`.
- `count` : print `open=<a> done=<b>`.
Any other command prints `error: unknown command <word>`. Blank lines are ignored. A command missing its argument (like `done` alone) prints `error: missing argument`.
