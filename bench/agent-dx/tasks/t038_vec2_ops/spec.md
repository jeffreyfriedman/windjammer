Implement a 2D integer vector type with operator overloading (or the closest equivalent in your language). Each input line is `ax ay OP bx by` with OP one of:

- `+` : component-wise sum, printed `(x, y)`
- `-` : component-wise difference, printed `(x, y)`
- `*` : dot product, printed as a plain integer
- `x` : 2D cross product `ax*by - ay*bx`, printed as a plain integer
- `s` : scale vector a by the integer b's first component `bx` (the `by` value is ignored), printed `(x, y)`
- `==` : `true` or `false`
Negative numbers print with a minus sign inside the parentheses, e.g. `(-3, 4)`. Values fit comfortably in i64.
