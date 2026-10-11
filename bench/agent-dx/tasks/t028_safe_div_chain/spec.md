Each input line is a chain `n0 / n1 / n2 / ...` of signed 64-bit integers separated by ` / ` (space, slash, space), with at least one number. Evaluate left to right with integer division truncating toward zero: `((n0 / n1) / n2) ...`.

Print the result. If a divisor (the k-th divisor, counting from 1) is zero print `error: division by zero at step k`; if a division would overflow (the minimum i64 divided by -1) print `error: overflow at step k`. Stop at the first error in the chain. A line with a single number just prints it.
