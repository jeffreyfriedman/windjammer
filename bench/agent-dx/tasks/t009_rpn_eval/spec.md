Each input line is a Reverse Polish Notation expression: tokens separated by single spaces, each token either a decimal integer with an optional leading `-` (no `+` sign, no decimals) or one of the operators `+ - * /`. Evaluate each line with 64-bit signed integers; `/` truncates toward zero. Blank lines are ignored.

Print the single result, or exactly one error line:
- `error: underflow` if an operator finds fewer than two operands,
- `error: divzero` if a division has divisor 0,
- `error: leftover` if evaluation finishes with more than one value on the stack (or with an empty stack),
- `error: token` if a token is neither an integer nor an operator.
The first error encountered, scanning left to right, wins. Inputs never overflow 64 bits.
