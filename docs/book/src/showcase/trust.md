# How the interpreter stays honest

<!-- feature: showcase-trust -->

A fast interpreter is only useful if it means the same thing as the compiled program. Windjammer treats that as a property
to be tested continuously, not a promise: the interpreter and the compiled backend are run against the same programs, and
any difference is a recorded finding that has to shrink.

## What is compared

**This book.** Every program in it runs interpreted and compiled, and the two outputs must be equal. The number of programs,
counting the ones that run only in the interpreter:

<!-- wj:gen name=metric key=book.examples -->
133
<!-- wj:end -->

**The specification.** Each rule in the language specification has an executable example with an expected output derived by
hand from the stated semantics, never copied from an implementation. The number of example files:

<!-- wj:gen name=metric key=spec.example_files -->
294
<!-- wj:end -->

**The conformance corpus.** A set of programs is run on every backend and the outputs compared. The number of programs, with
the pairs that do not pass listed on the [compile targets](../reference/targets.md) page:

<!-- wj:gen name=metric key=conformance.programs -->
156
<!-- wj:end -->

**The drift corpus.** Real programs written by someone other than the interpreter's authors, taken from the
windjammer-ecosystem repository, run under the tree-walking interpreter, the resolved engine and the compiled
backend, with each result compared. The number of cases:

<!-- wj:gen name=metric key=drift.corpus_cases -->
1946
<!-- wj:end -->

## Differences are ledgers that shrink

A known difference between the interpreter and the compiled backend goes into a ledger with a minimal program that shows
it. A fix makes the ledger entry stale and the gate fails until the line is deleted, so the list only gets shorter. This
book has such a ledger of its own: every row of "Found while writing the book" in
[Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) has a repro and a test that fails when the bug is fixed.

## The contract

The promise itself is written down in the repository (`docs/testing/INTERPRETER_CONTRACT.md`): for any program the
interpreter accepts, its standard output, its exit status and the source location of a panic equal the compiled
program's. [How this book is tested](../appendix/how-this-book-is-tested.md) shows the same machinery applied to
documentation.
