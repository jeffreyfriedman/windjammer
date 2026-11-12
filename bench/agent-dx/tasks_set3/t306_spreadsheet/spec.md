Implement a small spreadsheet. The input holds one command per line; empty lines are ignored. Cells are named by a column letter `A` to `J` followed by a row number 1 to 20 without leading zeros (`A1`, `J20`; `A0`, `A21`, `K1`, `a1` and `A01` are not cells). All values are integers.

Commands (a command word is followed by single spaces):

- `set CELL CONTENT` stores CONTENT, the rest of the line after `set CELL ` (non-empty, may contain spaces), in the cell. If CONTENT starts with `=` it is a formula. Otherwise, if it is an optional `-` followed by 1 to 9 digits, it is a number. Anything else is text.
- `clear CELL` empties the cell (clearing an empty cell is fine).
- `get CELL` prints `CELL = V`.
- `dump` prints `CELL = V` for every non-empty cell ordered by column letter, then by row number, or `(empty sheet)` when there are none.

V is the cell's current value: a number in decimal, the text itself, `(empty)` for an empty cell (only possible with `get`), or an error code (below). Any other line, an invalid cell name, or a wrong number of fields prints `error: bad command`.

Formulas. After `=` comes an expression; spaces between tokens are ignored. Grammar:

```
expr    = term { ('+' | '-') term }
term    = unary { ('*' | '/') unary }
unary   = '-' unary | primary
primary = NUMBER | CELL | FUNC '(' arg { ',' arg } ')' | '(' expr ')'
arg     = CELL ':' CELL | expr
```

NUMBER is 1 to 9 digits. FUNC is one of `SUM`, `MIN`, `MAX`, `COUNT` (one or more args) or `ABS` (exactly one arg, which must not be a range). A range names the rectangle spanned by its two corners in either order. A range may only appear directly as a function argument. Any syntax problem (unknown function or word, bad cell name, wrong arity, trailing input, empty expression) makes the `set` fail with `error: bad formula`; the cell keeps its old content.

A formula may use other cells, including cells that are set later. A formula must not depend, directly or through other cells or ranges, on its own cell. If `set` would create such a loop it is rejected with `error: cycle X -> Y -> ... -> X`, and the cell keeps its old content. To find the path, start at the cell being set (X) and depth-first search along dependencies. At each cell visit its dependencies (the cells and range members its formula mentions) in ascending order of column letter, then row number. Never revisit a cell already explored. Print the first path that returns to X, starting and ending with X. A formula that mentions its own cell gives `error: cycle X -> X`. The check is made before anything else changes, and a rejected `set` has no effect.

Values and errors. Values are recomputed from the current contents whenever they are read, so changing a cell changes every cell that depends on it, directly or not.

- An empty cell counts as 0 in arithmetic. A text cell used in arithmetic or ABS gives `#VALUE`.
- A formula cell whose result is a number shows the number. Errors propagate: the first error met while evaluating (left operand before right operand, arguments left to right, range members row by row, left to right within a row) is the result.
- Division truncates toward zero. Division by zero gives `#DIV0`. Any number whose absolute value exceeds 1000000000000 (10^12) gives `#OVERFLOW`; this is checked after every single operation. Arithmetic is exact: the product of two values of magnitude up to 10^12 may not fit in 64 bits and must still give `#OVERFLOW`.
- `SUM` adds the numbers in its arguments, `COUNT` counts them, `MIN` and `MAX` take the extreme. In a range, empty cells and text cells are skipped, numeric cells and numeric formula cells count, and an error cell gives that error. A scalar argument (an expression) always counts as one number. `MIN` and `MAX` with no numbers at all give `#VALUE`; `SUM` of none is 0, `COUNT` of none is 0.
- A cell reference used on its own as a scalar (not in a range) is an expression: empty gives 0, number gives the number, text gives `#VALUE`.

Exit status is 0 even when errors were printed.

Example:

```
set A1 10
set A2 =A1*3
set B1 =SUM(A1:A2)+MAX(1,A2)
set A3 =B1
get B1
set A1 =A3
set C1 =10/(A1-A1)
get C1
dump
```

prints

```
B1 = 70
error: cycle A1 -> A3 -> B1 -> A1
A1 = 10
A2 = 30
A3 = 70
B1 = 70
C1 = #DIV0
```

(`set A1 =A3` is rejected, so A1 stays 10; `C1` divides by zero.)
