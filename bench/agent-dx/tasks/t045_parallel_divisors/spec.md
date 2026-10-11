The first line holds `W N`: worker count W (1 <= W <= 8) and number of values N (1 <= N <= 40). The next N tokens are integers `n` with 1 <= n <= 10^9.

For every n compute the number of positive divisors, using trial division up to sqrt(n) (this is deliberately CPU-bound). Distribute the values over W workers however you like (a pool pulling from a shared queue, static slices, ...) but the output must list the results in INPUT order regardless of completion order: one line `<n> <divisor_count>` per value.
