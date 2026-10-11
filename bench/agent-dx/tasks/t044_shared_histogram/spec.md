The first line holds `W N`: the worker count (1 <= W <= 8) and the number of values N (0 <= N <= 50000). The next N whitespace-separated tokens are integers in 0..9.

Worker w (0-based) handles the values at positions i (0-based) with i % W == w. All workers update ONE shared histogram of ten counters protected by a lock / mutex / atomic (or an equivalent safe shared-state mechanism), and also count how many values they processed themselves.

Print `hist: <c0> <c1> ... <c9>`, then `worker w: <count>` for each worker in order, then `checksum: <sum over d of d * c_d>`.
