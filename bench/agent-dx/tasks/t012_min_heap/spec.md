Implement a min-priority-queue of (priority, label) pairs where priority is an integer and label a single word without spaces. Commands, one per line:

- `push p label` : insert. Print nothing.
- `pop` : remove and print `p label` of the entry with the smallest priority; among equal priorities the entry inserted EARLIEST comes first (FIFO). Print `empty` if there are none.
- `peek` : like pop without removing.
- `size` : print the number of entries.

Do not use a sorted insertion that is O(n) per operation for large inputs: the hidden tests include up to 2000 operations (but any correct algorithm passes).
