The first line holds an integer T. Each of the next T lines holds a non-negative integer `n` (0 <= n <= 10^18).

For each `n` print `<digit_sum> <digital_root> <reversed> <palindrome>` where `digital_root` is obtained by repeatedly summing digits until a single digit remains, `reversed` is the number with its decimal digits reversed (leading zeros dropped, so 120 -> 21), and `palindrome` is `yes` if the decimal representation reads the same backwards, else `no`.
