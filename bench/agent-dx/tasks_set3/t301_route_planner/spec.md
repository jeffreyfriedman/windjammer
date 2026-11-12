The input is a list of commands, one per line. Fields are separated by single spaces. Node names are non-empty tokens without spaces (letters, digits, underscore). Empty lines are ignored. Commands are processed in order, and the graph only contains the edges added before a query.

- `edge A B W` adds a directed edge from A to B with weight W. W is written with digits only (no sign) and must be between 1 and 1000000 inclusive. If an edge from A to B already exists, the smaller of the two weights is kept. Self edges (A equals B) are accepted. Both A and B become known nodes.
- `path A B` asks for the cheapest route from A to B and prints exactly one line:
  - if A is not a known node, `unknown node A`; otherwise if B is not a known node, `unknown node B`
  - if A and B are the same known node, `0 A`
  - if B cannot be reached from A, `unreachable`
  - otherwise `COST N1>N2>...>Nk`, where COST is the minimum total weight and N1 is A, Nk is B and the nodes are the route in order.

Tie-break: when several routes have the same minimum cost, print the route whose node sequence is lexicographically smallest, comparing node names as byte strings, element by element from the start. For example `n10` sorts before `n2`, and `B` sorts before `a`.

Any other line (unknown command, wrong number of fields, invalid weight, and so on) prints `bad command: ` followed by the original line, and processing continues.

The exit status is always 0. A line with only spaces is not an empty line: it is a bad command.

Notes:

- Fields are separated by exactly one space, so a line with a leading, trailing or doubled space has an empty field and is a bad command.
- Weights are small enough that all costs fit comfortably in 64 bits.
- Edges are directed: `edge a b 1` does not make `a` reachable from `b`.
- Routes never repeat a node (all weights are positive, so a repeated node could only cost more).
- Queries only see edges added earlier in the input; a later `edge` line can change the answer of a later `path` line.
- The `COST` is printed in decimal with no padding, and the nodes are joined by `>` with no spaces.
- The input may end without a final newline; the last line is still processed.

Example:

```
edge s a 1
edge s b 1
edge a t 1
edge b t 1
path s t
path t s
path s q
```

prints

```
2 s>a>t
unreachable
unknown node q
```
