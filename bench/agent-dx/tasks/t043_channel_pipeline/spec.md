Build a pipeline of concurrent stages connected by channels (or queues, streams, generators; your language's idiom). The first line is `N` followed by a line containing a stage list: space-separated stage tokens (possibly none: the second line may be empty). The source stage emits the integers 1..N in order. Each stage token transforms the stream:

- `add:K` : x -> x + K
- `mul:K` : x -> x * K
- `gt:K` : keep only x > K
- `even` : keep only even x
- `odd` : keep only odd x
- `dedupe3` : drop an item if it equals either of the previous two ITEMS THAT REACHED THIS STAGE

Integers K may be negative. Order of items must be preserved through the pipeline. Print on the first line the number of items that reach the end, on the second line the items separated by single spaces (empty line if none), on the third `sum=<sum of items>`. 0 <= N <= 2000; all values fit in i64.
