The first line holds an integer T. Each of the next T lines holds `n m` (0 <= n <= 10^18, 1 <= m <= 10^9).

Print `F(n) mod m` for each line, where F(0)=0, F(1)=1, F(k)=F(k-1)+F(k-2). The input is large enough that you need an O(log n) method. Intermediate products can exceed 32 bits; take care with overflow.
