The input line is `P K C` : P producers (1..6), each producing K items (0..200), and C consumers (1..4).

Producer p (0-based) produces the strings `p<p>-<j>` for j from 1 to K and sends them into one shared bounded queue/channel (capacity 4 is fine). C consumers receive from the queue until it is closed and drained. Each consumer counts how many items it received and the total number of characters in them. Because scheduling is nondeterministic, only report scheduling-independent facts:

Print `items: <P*K>`, `chars: <total characters over all items>`, then the lexicographically sorted FIRST three items received overall (sort all items bytewise ascending, take the first 3; fewer if fewer exist), one per line prefixed by `first: `. Finally print `consumers_total: <sum of all consumers' counts>` which must equal the item count.
