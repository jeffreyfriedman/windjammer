Implement lazy sequences via your language's iterator abstraction (a custom iterator type/generator, not precomputed tables). Each input line is `KIND MODE N`:

- KIND is one of `fib` (0,1,1,2,3,5,...), `squares` (1,4,9,16,...), `primes` (2,3,5,7,...), `evens` (0,2,4,6,...), `triangular` (1,3,6,10,...).
- MODE `take`: print the first N items of the sequence. MODE `below`: print all items strictly less than N.
- MODE `sum`: print only the sum of the first N items.

Print items separated by single spaces on one line (an empty result prints an empty line). Sequences can be infinite, so your solution must stop lazily. N is between 0 and 60 for `take`/`sum` and 0..100000 for `below`. Values fit in i64.
