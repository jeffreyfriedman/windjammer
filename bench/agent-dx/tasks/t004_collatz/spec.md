The first line holds an integer T. Each of the next T lines holds one integer `n` (1 <= n <= 1000000).

Apply the Collatz rule repeatedly (if n is even, n = n/2, else n = 3n+1) until n reaches 1. For each input print `<steps> <peak>`: the number of steps taken and the largest value reached (including the starting value). Use 64-bit integers.
