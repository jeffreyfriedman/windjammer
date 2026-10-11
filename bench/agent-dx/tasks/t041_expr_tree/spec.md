Each input line is a prefix (Lisp-style) expression: either an integer literal (possibly negative), or `(OP a b)` with OP in `+ - *` and sub-expressions a, b, or `(neg a)`. Tokens are separated by single spaces; parentheses touch their neighbors only as shown (`(+ 1 (* 2 3))`). Inputs are always well formed. Parse into a tree (use an enum / sum type / class hierarchy) and print four values on one line, separated by single spaces:

1. the evaluated value (64-bit signed),
2. the depth of the tree (a literal has depth 1),
3. the number of nodes,
4. the fully parenthesized infix rendering: a literal prints as is, `(+ a b)` as `(A + B)`, `(neg a)` as `-(A)`, with `-` and `*` printed similarly. Because the rendering contains spaces, print it as the LAST field, i.e. everything after the third space is the rendering.
