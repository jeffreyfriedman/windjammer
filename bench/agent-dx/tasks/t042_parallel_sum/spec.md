The first line holds `W N`: the number of workers W (1 <= W <= 16) and the number of values N (0 <= N <= 100000). The next tokens (any whitespace) are N signed integers with absolute value <= 10^9.

Split the values into W contiguous chunks in input order: with q = N / W (integer division) and r = N % W, the first r chunks have q+1 values and the remaining chunks have q values (chunks may be empty). Compute each chunk's sum in its own worker (use threads, goroutines, tasks, or the closest concurrency primitive your language offers) and collect the results over a channel or join handles.

Print one line `chunk i: <sum>` per chunk (i from 0) in chunk order regardless of which worker finishes first, and a final line `total: <sum>`. Sums fit in 64-bit signed integers.
