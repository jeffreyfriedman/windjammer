Simulate a line editor. The buffer is a list of text lines, initially empty, indexed from 0. The input holds one command per line, fields separated by single spaces. Empty input lines are ignored.

Commands:

- `insert N TEXT` inserts TEXT as a new line so that it ends up at index N (N from 0 to the current line count inclusive).
- `delete N` removes the line at index N.
- `replace N TEXT` replaces the line at index N with TEXT.
- `swap A B` exchanges the lines at indexes A and B (A may equal B).
- `undo` reverts the most recent edit that has not been undone yet.
- `redo` re-applies the most recently undone edit.
- `print` prints the buffer: `(empty)` when there are no lines, otherwise one line `N: TEXT` per buffer line in index order.

TEXT is everything after the second space of the line (it may itself contain spaces) and must not be empty. Indexes N, A, B are written with digits only and have at most 6 digits.

The four editing commands (`insert`, `delete`, `replace`, `swap`) are edits. History rules:

- A successful edit is pushed on the undo history and clears the redo history. An edit that fails (see below) changes nothing, including the history.
- `undo` and `redo` themselves are not edits. They move one edit between the undo history and the redo history.
- The undo history keeps only the 5 most recent edits; when a sixth is pushed, the oldest is dropped and can no longer be undone. The redo history has no limit.

Errors are reported on standard output as a single line each and processing continues. They are checked in this order:

1. first word is not one of the seven commands: `error: unknown command`
2. wrong number of fields for the command, an index that is not digits-only (or has more than 6 digits), or empty TEXT: `error: bad arguments`. `undo`, `redo` and `print` take no arguments, `delete` takes one, `swap` takes two indexes, `insert` and `replace` take an index and TEXT.
3. an index outside the valid range (for `delete`, `replace` and `swap` valid indexes are 0 to count-1): `error: bad index`
4. `undo` with an empty undo history: `error: nothing to undo`; `redo` with an empty redo history: `error: nothing to redo`

The exit status is 1 if at least one error line was printed, otherwise 0. Output is produced as commands are processed.

Model each edit as a command object that knows how to apply and revert itself, so that undo and redo work uniformly for all four kinds.

Example:

```
insert 0 hello world
insert 1 bye
swap 0 1
print
undo
print
delete 5
```

prints

```
0: bye
1: hello world
0: hello world
1: bye
error: bad index
```

and exits with status 1.
