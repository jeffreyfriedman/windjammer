Build a reusable memoizing wrapper that caches results of a pure function keyed by its argument and counts hits and misses. Use it for these functions (each input line is `NAME ARG`; ARG is a non-negative integer):

- `fib ARG` : Fibonacci F(ARG) modulo 1000000007 (ARG <= 80)
- `collatz ARG` : number of Collatz steps from ARG to 1 (ARG >= 1, ARG <= 100000)
- `tri ARG` : ARG*(ARG+1)/2

Each distinct (NAME, ARG) pair is a separate cache entry. For every line print `<value> hit` if that exact pair was requested before on an earlier line, otherwise `<value> miss`. After the last line print `hits=<h> misses=<m>`. (Recursive internal calls need not be counted: only the lines of the input.) A line with an unknown NAME prints `unknown function` and counts as neither a hit nor a miss.
