Resolve an install order for packages with versioned dependencies. The input has one declaration per line, fields separated by single spaces. Empty lines are ignored but still count when numbering lines (starting at 1).

**Line splitting (exact, no trimming).** Fields are obtained by splitting the line on single space characters, exactly. A leading, trailing or doubled space therefore produces an empty field, and the line is handled like any other line with a wrong number of fields or a malformed token (for example ` open a 1` is not `open a 1`). Only a line of length zero is an empty line: a line that holds only spaces is not.

- `pkg NAME VERSION` declares a package with a single version.
- `dep NAME DEP OP VERSION` says package NAME requires package DEP whose version satisfies the constraint `OP VERSION`. OP is one of `>=`, `<=`, `=`, `>`, `<`. The same NAME and DEP pair may have several `dep` lines; all must hold.
- `install NAME` asks for NAME (and everything it needs) to be installed. Several `install` lines are allowed.

Names are non-empty tokens without spaces. VERSION is digits only with at most 6 digits and is compared as a number. Declarations may appear in any order.

Processing happens in three phases; every problem becomes one `error: ...` line and the phases append to one shared list of error lines:

1. In line order: a line that is not a well-formed declaration (unknown first word, wrong number of fields, bad number, bad OP) gives `error: bad line L` with L the line number. A second `pkg` line for a name that was already declared gives `error: duplicate package NAME`; the first declaration is kept and the later one ignored.
2. In line order: each well-formed `dep` line whose NAME is not a declared package gives `error: dep for unknown package NAME` and is ignored.
3. For each `install NAME` line in order: if NAME is not declared, `error: unknown package NAME`. Otherwise run a depth-first visit from NAME unless NAME was already fully visited.

The visit of package U works like this. Mark U as in progress and remember the chain of in-progress packages. Take the distinct package names that U depends on in ascending byte order. For each such DEP:

- if DEP is not declared: `error: missing dependency DEP of U`
- otherwise, for each constraint of the pair U, DEP in input order that DEP's version does not satisfy: `error: U needs DEP OP V but found W` (W is DEP's version; the constraint is written as OP, a space, and V as a plain decimal number without leading zeros)
- if DEP is declared and in progress: `error: cycle P1 -> P2 -> ... -> P1`, listing the chain from DEP up to U and then DEP again (a package depending on itself gives `error: cycle U -> U`). Do not descend.
- if DEP is declared and not yet visited, visit it (even when one of its constraints failed).

When all dependencies of U are handled U is finished and appended to the install order. A finished package is never visited again.

Output: if there is at least one error line, print all error lines in the order they were produced and exit with status 1; print no install lines. Otherwise print `install NAME VERSION` for each package in the install order and exit with status 0 (no `install` lines in the input prints nothing).

Example:

```
pkg app 1
pkg lib 3
pkg core 2
dep app lib >= 2
dep app core = 2
dep lib core < 5
install app
```

prints

```
install core 2
install lib 3
install app 1
```
