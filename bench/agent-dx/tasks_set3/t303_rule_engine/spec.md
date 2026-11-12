The input describes a shopping cart and an ordered list of discount rules, one per line, fields separated by single spaces. Empty lines are ignored but still count when numbering lines (line numbers start at 1). All amounts are integers in cents.

Line kinds:

- `item SKU QTY UNIT`: a cart line. QTY is at least 1, UNIT at least 0.
- `rule percent TARGET P`: P percent off. P is 0 to 100.
- `rule fixed TARGET C`: C cents off each matching line. C is at least 0.
- `rule bogo TARGET N M`: buy N get M free. N and M are at least 1.
- `rule threshold T P`: if the running cart total is at least T, take P percent off the whole cart. T is at least 0, P is 0 to 100.

TARGET is either a SKU or `*` (all lines). A SKU is any token without spaces. Every number is written with digits only (no sign) and has at most 9 digits. Any other line (unknown first word or rule kind, wrong number of fields, bad or out-of-range number) is invalid: it is reported as `error line L: ` followed by the original line, where L is its line number, and otherwise ignored. All error lines are printed first, in input order.

Evaluation. Read the whole input first; items are kept in input order (several items may share a SKU and are separate cart lines), and valid rules are numbered 1, 2, ... in input order. Each cart line starts with `current = QTY * UNIT` and the cart discount starts at 0. Then apply the rules in order. Every division rounds down.

- percent: for each matching line, `d = current * P / 100`; the line's current drops by d.
- fixed: for each matching line, `d = min(current, C)`; the line's current drops by d.
- bogo: for each matching line, `free = (QTY / (N + M)) * M` units are free, `d = min(current, free * UNIT)`; the line's current drops by d.
- threshold: `running = (sum of all current) - cart discount`. If `running >= T`, then `d = running * P / 100` is added to the cart discount, otherwise `d = 0`.

The saving of a rule is the sum of its d values (0 when nothing matches). Rules later in the list see the effect of earlier ones.

Output after the error lines: one line `item SKU BASE FINAL` per cart line in input order (BASE is `QTY * UNIT`, FINAL the line's current after all rules), then one line `rule I KIND SAVING` per valid rule in order (KIND is the rule kind word), then `total X` where X is `(sum of all current) - cart discount`, but never below 0.

Exit status: 1 if at least one error line was printed, otherwise 0. The report is printed in either case.

Model the four rule kinds as separate types behind one common interface (for example a trait object stored in a list), so a new kind could be added without touching the evaluation loop.

Example:

```
item apple 6 100
item pear 2 250
rule percent apple 10
rule bogo pear 1 1
rule threshold 500 10
```

prints

```
item apple 600 540
item pear 500 250
rule 1 percent 60
rule 2 bogo 250
rule 3 threshold 79
total 711
```
